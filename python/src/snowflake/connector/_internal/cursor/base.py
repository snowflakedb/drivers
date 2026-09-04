"""Shared cursor base mixin for sync and async cursor implementations."""

from __future__ import annotations

import abc
import ctypes
import re
import warnings

from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING, Any, cast

from ..._common.extras import check_dependency, pandas, pyarrow
from ...constants import SessionParameterName, StatementParameterName
from ...errors import Error, ErrorValue, InterfaceError, NotSupportedError, ProgrammingError
from ..api_client.client_api import core_driver
from ..binding_converters import (
    BindingConverterBase,
    ClientSideBindingConverter,
    CsvBindingConverter,
    JsonBindingConverter,
    ParamStyle,
    parse_stage_binding_threshold,
)
from ..config_utils import create_config_setting
from ..decorators import api_telemetry, backward_compatibility, pep249, snowpark_compat
from ..errorcode import ER_FAILED_TO_REWRITE_MULTI_ROW_INSERT, ER_INVALID_VALUE
from ..errorhandler import ErrorHandlerMixin
from ..protobuf_gen.database_driver_v1_pb2 import BinaryDataPtr, ConfigSetting, QueryBindings, StatementHandle
from ..text_utils import extract_values_clause
from .decorators import requires_open
from .query_result import MultiStatementQueryResultState, QueryResult
from .result_metadata import QueryResultStats, ResultMetadata


if TYPE_CHECKING:
    from ...aio.connection import Connection as AsyncConnection
    from ...connection import Connection
    from .result_metadata import ResultMetadataV2


class CursorBaseMixin(ErrorHandlerMixin, abc.ABC):
    """Zero-I/O cursor members shared by sync and async base cursor classes."""

    _BARE_DESC_SQL_RE = re.compile(r"desc(?:ribe)?\s+([\w_]+)\s*;?\s*$", re.IGNORECASE)
    _INSERT_SQL_RE = re.compile(r"^insert\s+into", re.IGNORECASE)
    _COMMENT_SQL_RE = re.compile(r"/\*.*\*/")

    # Set by subclass ``__init__`` before ``super().__init__()``.
    _connection: Connection | AsyncConnection
    _closed: bool
    _messages: list[tuple[type[Exception], ErrorValue]]
    _errorhandler: Callable[..., None]
    _arraysize: int
    _query_result: QueryResult
    _rownumber: int
    _multi_statement: MultiStatementQueryResultState | None
    _statement_parameters: dict[str, Any]
    _binding_data: None | bytes

    def __init__(self) -> None:
        self._closed = False
        self._messages = []
        self._errorhandler = Error.default_errorhandler

        # -- PEP 249 cursor configuration (persists for cursor lifetime) --
        self._arraysize = 1

        # -- Query result state (replaced on execute, mutated on reset/consume) --
        self._query_result = QueryResult()

        # Cursor navigation position — mutable to avoid allocation per fetchone
        self._rownumber = -1

        # -- Multi-statement navigation (set by _handle_multi_statement_response, cleared on reset) --
        self._multi_statement = None

        # -- Statement parameters (persists until explicitly changed) --
        self._statement_parameters = {}

        # Keep binding data reference to prevent garbage collection while Rust uses it
        self._binding_data = None

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
    @pep249
    @api_telemetry
    def description(self) -> list[ResultMetadata] | None:
        """
        Read-only attribute describing the result columns of a query.

        Returns a sequence of 7-item tuples, each containing:
        - name: Column name (str)
        - type_code: Integer type code (int)
        - display_size: Display size in characters (int | None)
        - internal_size: Internal size in characters (int | None)
        - precision: Precision for numeric types (int | None)
        - scale: Scale for numeric types (int | None)
        - null_ok: True if column can contain NULLs (bool | None)

        Returns None if no query has been executed or if the query didn't produce a result set.
        """
        return self._query_result.description

    @property
    @snowpark_compat
    @backward_compatibility
    def _description_internal(self) -> list[ResultMetadataV2] | None:
        """New-format column metadata for the last executed statement.

        Snowpark probes for this attribute and falls back to :attr:`description`
        when it is absent, which loses ``fields`` and ``vector_dimension``.
        """
        return self._query_result.description_v2

    @property
    @pep249
    @api_telemetry
    def rowcount(self) -> int | None:
        """
        Read-only attribute specifying the number of rows that the last
        .execute*() produced or affected.

        Returns:
            int: Number of rows affected, or None if not determined
        """
        return self._query_result.rowcount

    @property
    @pep249
    @api_telemetry
    def arraysize(self) -> int:
        """Number of rows to fetch at a time with .fetchmany(). Defaults to 1."""
        return self._arraysize

    @arraysize.setter
    @api_telemetry
    def arraysize(self, value: int) -> None:
        self._arraysize = int(value)

    @property
    @pep249
    @api_telemetry
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    @api_telemetry
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    # ------------------------------------------------------------------
    # Execution metadata
    # ------------------------------------------------------------------

    @property
    @api_telemetry
    def query(self) -> str | None:
        """
        Read-only attribute containing the SQL text of the last executed or described query.

        Returns:
            str | None: The SQL query string, or None if no query has been executed or described
        """
        return self._query_result.query

    @property
    @api_telemetry
    def sfqid(self) -> str | None:
        """
        Read-only attribute containing the Snowflake Query ID for the last executed or described query.

        Returns:
            str | None: Snowflake Query ID (UUID format), or None if no query has been executed or described
        """
        return self._query_result.sfqid

    @property
    @api_telemetry
    def request_id(self) -> str | None:
        """
        Read-only attribute containing the client-generated ``requestId`` of the last query submission.

        Returns:
            str | None: Client request UUID, or None when unavailable.
        """
        return self._query_result.request_id

    @property
    @snowpark_compat
    @backward_compatibility(recommendation="Use the public `request_id` property instead.")
    def _request_id(self) -> str | None:
        """Private alias of :attr:`request_id`, kept for Snowpark's expected attribute name."""
        return self.request_id

    @property
    @api_telemetry
    def stats(self) -> QueryResultStats:
        """Returns detailed row-level statistics for DML operations."""
        return self._query_result.stats

    @property
    @pep249
    @api_telemetry
    def rownumber(self) -> int | None:
        """The current 0-based index of the cursor in the result set, or ``None`` if indeterminate."""
        return self._rownumber if self._rownumber >= 0 else None

    @property
    @api_telemetry
    def sqlstate(self) -> str | None:
        """The SQLSTATE code of the last executed operation."""
        return self._query_result.sqlstate

    @property
    @api_telemetry
    def multi_statement_parent_sfqid(self) -> str | None:
        """
        Read-only attribute containing the parent Snowflake Query ID for multi-statement queries.

        Returns:
            str | None: Parent query ID for multi-statement, or None for single statements.
        """
        return self._multi_statement.parent_qid if self._multi_statement else None

    @property
    @api_telemetry
    def multi_statement_savedIds(self) -> list[str]:
        """
        Read-only attribute containing child query IDs for multi-statement queries.

        Returns:
            list[str]: List of child query IDs (empty list for single statements).
        """
        return self._multi_statement.child_query_ids if self._multi_statement else []

    @property
    @api_telemetry
    def is_file_transfer(self) -> bool:
        """Whether the last executed command was a PUT or GET file transfer."""
        return self._query_result.is_file_transfer

    def _stage_binding_threshold(self) -> int:
        raw = self._connection._session_parameters[SessionParameterName.CLIENT_STAGE_ARRAY_BINDING_THRESHOLD]
        return parse_stage_binding_threshold(raw)

    def _should_use_csv_binding(self, parameters: Sequence[Any], threshold: int, query: str = "") -> bool:
        """Return whether array bindings should be uploaded as CSV to a stage.

        Stage binding applies only to INSERT statements with multi-row (array)
        bindings whose cell count meets ``CLIENT_STAGE_ARRAY_BINDING_THRESHOLD``.
        The server rejects CSV stage bindings for non-INSERT queries, matching
        the legacy connector's behavior of gating on INSERT.
        Scalar binds always use inline JSON regardless of threshold.

        """
        if not query.lstrip().upper().startswith("INSERT"):
            return False
        if not any(isinstance(value, list) for value in parameters):
            return False
        effective_cells = BindingConverterBase.effective_binding_cells(parameters)
        if effective_cells == 0 or threshold <= 0:
            return False
        return effective_cells >= threshold

    def _build_query_bindings(self, parameters: Sequence[Any], query: str = "") -> QueryBindings | None:
        """Serialize parameters and build a QueryBindings protobuf message.

        Converts Python parameter values to JSON or CSV, then wraps the result
        in a zero-copy BinaryDataPtr so the Rust core can read the bytes
        directly from Python memory.

        The encoded bytes are stored on ``self._binding_data`` to prevent
        garbage collection while Rust holds the pointer.

        Returns:
            QueryBindings with the serialized JSON or CSV, or None if parameters
            serialize to nothing (e.g. empty list).
        """
        threshold = self._stage_binding_threshold()
        use_csv = self._should_use_csv_binding(parameters, threshold, query)

        if use_csv:
            binding_bytes = CsvBindingConverter.serialize_parameters_to_csv(parameters)
            if binding_bytes is None:
                return None
        else:
            json_str, _ = JsonBindingConverter.serialize_parameters(parameters)
            if json_str is None:
                return None
            binding_bytes = json_str.encode("utf-8")

        self._binding_data = binding_bytes
        length = len(binding_bytes)

        ptr_value = ctypes.cast(ctypes.c_char_p(self._binding_data), ctypes.c_void_p).value
        if ptr_value is None:
            raise RuntimeError("Failed to obtain memory pointer for binding data")

        ptr_bytes = ptr_value.to_bytes(8, byteorder="little", signed=False)
        binary_data_ptr = BinaryDataPtr(value=ptr_bytes, length=length)
        if use_csv:
            return QueryBindings(csv=binary_data_ptr)
        return QueryBindings(json=binary_data_ptr)

    def _prepare_query(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None,
        _force_qmark_paramstyle: bool = False,
    ) -> tuple[str, Sequence[Any] | None]:
        """Prepare query and server-side binding parameters based on paramstyle.

        Args:
            operation: SQL statement
            parameters: Parameters to bind (sequence or dict)
            _force_qmark_paramstyle: If True, treat the call as qmark
                regardless of the connection's configured paramstyle. Used by
                callers (e.g. snowflake.core) that emit ``?`` placeholders
                while connections may default to pyformat.

        Returns:
            Tuple of (query string, server-side binding parameters or None)

        Raises:
            ProgrammingError: If dict parameters used with server-side binding
        """
        match = self._BARE_DESC_SQL_RE.match(operation)
        if match:
            warnings.warn(
                "Bare 'DESC <table>' and 'DESCRIBE <table>' statements are deprecated "
                "and will be removed in a future release; use 'DESC TABLE <table>' instead.",
                DeprecationWarning,
                stacklevel=4,
            )
            operation = f"describe table {match.group(1)}"

        if parameters is None:
            return operation, None

        paramstyle = ParamStyle.QMARK if _force_qmark_paramstyle else self._connection.paramstyle

        if paramstyle.is_client_side():
            # format paramstyle only supports positional params (%s), not named params
            if paramstyle == ParamStyle.FORMAT and isinstance(parameters, dict):
                raise ProgrammingError(
                    msg="Dict parameters not supported with format paramstyle. "
                    "Use pyformat paramstyle for named parameters, or use a sequence.",
                    errno=ER_INVALID_VALUE,
                )
            # Client-side binding: interpolate parameters into SQL string
            query = ClientSideBindingConverter.interpolate_query(
                operation,
                parameters,
                interpolate_empty_sequences=self._connection._interpolate_empty_sequences,
            )
            return query, None
        else:
            # Server-side binding: qmark or numeric
            if isinstance(parameters, dict):
                raise ProgrammingError(
                    msg="Named parameters (dict) not supported with qmark/numeric paramstyle. "
                    "Use pyformat paramstyle for named parameters.",
                    errno=ER_INVALID_VALUE,
                )
            return operation, parameters

    def _format_query_for_log(self, query: str) -> str:
        return self._connection._format_query_for_log(query)

    def _prepare_call_proc_statement(self, procname: str, args: Any = None) -> tuple[str, Sequence[Any]]:
        """Validate ``callproc`` arguments and build the CALL statement."""
        if args is None:
            args = ()
        if isinstance(args, (str, bytes)):
            raise TypeError(f"callproc args must be a sequence (e.g. list or tuple), not {type(args).__name__}")
        if not isinstance(args, Sequence):
            raise TypeError(f"callproc args must be a sequence (e.g. list or tuple), not {type(args).__name__}")
        command = f"CALL {procname}({self._connection.paramstyle.placeholders(len(args))})"
        return command, args

    def _set_statement_parameter(self, key: str, value: Any) -> None:
        """Store a statement-level parameter for the next execute."""
        self._statement_parameters[key] = value

    @api_telemetry
    @requires_open
    def set_statement_parameter(self, key: str, value: Any) -> None:
        """Set a sticky statement-level parameter (e.g., MULTI_STATEMENT_COUNT).

        Persists across `execute()` calls on this cursor until explicitly
        changed. For per-call kwargs (one execute, no bleed across calls),
        use the `statement_parameters` channel in `execute()` instead.

        This must be called before execute() to take effect.

        Args:
            key: Parameter name (e.g., "MULTI_STATEMENT_COUNT").
            value: Parameter value.

        Raises:
            InterfaceError: If cursor is closed.

        Example:
            cursor.set_statement_parameter("MULTI_STATEMENT_COUNT", 3)
            cursor.execute("SELECT 1; SELECT 2; SELECT 3")
        """
        self._set_statement_parameter(key, value)

    def _build_array_binding_params(
        self,
        operation: str,
        seq_of_parameters: Sequence[Sequence[Any] | dict[str, Any]],
        first_params: Sequence[Any] | dict[str, Any],
    ) -> list[list[Any]]:
        """Validate uniform row widths and transpose rows to column-major bind arrays.

        Used by ``executemany`` server-side array binding (sequence params only;
        dict params are handled by the caller before reaching here).
        """
        # Dict params were handled by the caller; only sequences reach here.
        rows = cast("Sequence[Sequence[Any]]", seq_of_parameters)

        # Error code 251007 (ER_INVALID_VALUE) matches reference driver behavior
        first_len = len(first_params)
        for params in rows:
            if len(params) != first_len:
                raise InterfaceError(
                    msg=f"Bulk data size don't match. expected: {first_len}, got: {len(params)}, command: {operation}",
                    errno=ER_INVALID_VALUE,
                )

        # Transpose from row-major to column-major format
        # Input:  [(row1_col1, row1_col2), (row2_col1, row2_col2), ...]
        # Output: [[row1_col1, row2_col1, ...], [row1_col2, row2_col2, ...]]
        return [[row[col_idx] for row in rows] for col_idx in range(first_len)]

    def _rewrite_multirow_insert(
        self,
        operation: str,
        seq_of_parameters: Sequence[Sequence[Any] | dict[str, Any]],
    ) -> str | None:
        """Rewrite a client-side-bound INSERT into a single multi-row INSERT.

        Used by ``executemany`` for pyformat/format paramstyles so bulk
        inserts issue one HTTP request instead of one per row, mirroring the
        reference connector's rewrite. Returns ``None`` when ``operation`` is
        not a rewritable ``INSERT INTO ... VALUES (...)`` statement (e.g.
        UPDATE, DELETE, MERGE) — callers fall back to per-row execution.
        """
        stripped = operation.strip(" \t\n\r")
        if not self._INSERT_SQL_RE.match(stripped):
            return None

        command_wo_comments = self._COMMENT_SQL_RE.sub("", stripped)
        fmt = extract_values_clause(command_wo_comments)
        if fmt is None:
            raise ProgrammingError(
                msg=(
                    "executemany() failed to rewrite INSERT as multi-row: no VALUES clause found."
                    " Ensure the statement has the form 'INSERT INTO ... VALUES (...)'."
                ),
                errno=ER_FAILED_TO_REWRITE_MULTI_ROW_INSERT,
            )

        values = [ClientSideBindingConverter.interpolate_query(fmt, params) for params in seq_of_parameters]
        return stripped.replace(fmt, ",".join(values), 1)

    def _collect_statement_params(
        self,
        *,
        skip_upload_on_content_match: bool,
        num_statements: int | None = None,
        statement_params: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """Collect per-call statement parameters for a single execute().

        Merges the caller-supplied ``_statement_params`` bag (opaque passthrough,
        e.g. ``DATE_INPUT_FORMAT``) with the driver's known per-call fold-ins
        (``num_statements`` → MULTI_STATEMENT_COUNT, ``skip_upload_on_content_match``
        → SKIP_UPLOAD_ON_CONTENT_MATCH). Scoped to one call and never written to
        the cursor's sticky `_statement_parameters`.
        """
        # Seed from the caller's bag, then apply the driver fold-ins on top so
        # they win on a key collision — matches legacy, which spreads
        # ``{**_statement_params, "MULTI_STATEMENT_COUNT": num_statements}``.
        params: dict[str, Any] = dict(statement_params or {})
        if skip_upload_on_content_match:
            params[StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH] = True
        if num_statements is not None:
            params[StatementParameterName.MULTI_STATEMENT_COUNT] = num_statements
        return params

    def _build_statement_parameters_options(
        self, statement_parameters: dict[str, Any] | None = None
    ) -> dict[str, ConfigSetting]:
        """Build SetOptions from the cursor's sticky `_statement_parameters`
        merged with per-call `statement_parameters`. Per-call wins on key
        collision and is never persisted on the cursor.
        """
        merged_statement_params = {**self._statement_parameters, **(statement_parameters or {})}
        options: dict[str, ConfigSetting] = {}
        for key, value in merged_statement_params.items():
            try:
                setting = create_config_setting(value)
            except TypeError as err:
                raise ProgrammingError(msg=f"Cannot set parameter '{key}': {err}") from err
            if setting is not None:
                options[key] = setting
        return options

    def _apply_statement_parameters(
        self,
        stmt_handle: StatementHandle,
        statement_parameters: dict[str, Any] | None = None,
    ) -> None:
        """Apply sticky `_statement_parameters` merged with per-call
        `statement_parameters` via SetOptions RPC. Per-call wins on key
        collision and is never persisted on the cursor.
        """
        options = self._build_statement_parameters_options(statement_parameters)
        if not options:
            return
        core_driver.statement_set_options(stmt_handle=stmt_handle, options=options)

    # ------------------------------------------------------------------
    # Cursor state / navigation
    # ------------------------------------------------------------------

    @abc.abstractmethod
    def reset(self, closing: bool = False) -> None:
        """Release result-set resources; implemented by ``SnowflakeCursorBase``."""
        ...

    @api_telemetry
    def is_closed(self) -> bool:
        """
        Check if the cursor is closed.

        Returns:
            bool: True if closed, False otherwise
        """
        return self._closed or self._connection.is_closed()

    @pep249
    @api_telemetry
    def close(self) -> bool | None:
        """Close the cursor now.

        Returns whether the cursor was closed during this call.
        """
        try:
            if self._closed:
                return False
            self.reset(closing=True)
            self._closed = True
            del self._messages[:]
            return True
        except Exception:
            return None

    @pep249
    @api_telemetry
    def scroll(self, value: int, mode: str = "relative") -> None:
        """Scroll the cursor in the result set."""
        raise NotSupportedError("scroll is not supported")

    @pep249
    @api_telemetry
    def setinputsizes(self, sizes: Sequence[Any]) -> None:
        """Not supported."""
        return None

    @pep249
    @api_telemetry
    def setoutputsize(self, size: int, column: int | None = None) -> None:
        """Not supported."""
        return None

    @property
    @pep249
    @api_telemetry
    def lastrowid(self) -> None:
        """Snowflake does not support lastrowid; returns None per PEP 249."""
        return None

    # ------------------------------------------------------------------
    # Session parameter accessors
    # ------------------------------------------------------------------

    @property
    @api_telemetry
    def timestamp_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_OUTPUT_FORMAT`` parameter value."""
        return self._connection._session_parameters._get_string("TIMESTAMP_OUTPUT_FORMAT")

    @property
    @api_telemetry
    def timestamp_ltz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_LTZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return (
            self._connection._session_parameters._get_string("TIMESTAMP_LTZ_OUTPUT_FORMAT")
            or self.timestamp_output_format
        )

    @property
    @api_telemetry
    def timestamp_tz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_TZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return (
            self._connection._session_parameters._get_string("TIMESTAMP_TZ_OUTPUT_FORMAT")
            or self.timestamp_output_format
        )

    @property
    @api_telemetry
    def timestamp_ntz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_NTZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return (
            self._connection._session_parameters._get_string("TIMESTAMP_NTZ_OUTPUT_FORMAT")
            or self.timestamp_output_format
        )

    @property
    @api_telemetry
    def date_output_format(self) -> str | None:
        """The session's ``DATE_OUTPUT_FORMAT`` parameter value."""
        return self._connection._session_parameters._get_string("DATE_OUTPUT_FORMAT")

    @property
    @api_telemetry
    def time_output_format(self) -> str | None:
        """The session's ``TIME_OUTPUT_FORMAT`` parameter value."""
        return self._connection._session_parameters._get_string("TIME_OUTPUT_FORMAT")

    @property
    @api_telemetry
    def timezone(self) -> str | None:
        """The session's ``TIMEZONE`` parameter value."""
        return self._connection._session_parameters._get_string("TIMEZONE")

    @property
    @api_telemetry
    def binary_output_format(self) -> str | None:
        """The session's ``BINARY_OUTPUT_FORMAT`` parameter value (``HEX`` or ``BASE64``)."""
        return self._connection._session_parameters._get_string("BINARY_OUTPUT_FORMAT")

    # ------------------------------------------------------------------
    # Error handling
    # ------------------------------------------------------------------

    @property
    @pep249
    @api_telemetry
    def errorhandler(self) -> Callable:
        """PEP 249 error handler for this cursor."""
        return self._errorhandler

    @errorhandler.setter
    @api_telemetry
    def errorhandler(self, value: Callable | None) -> None:
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

    # ------------------------------------------------------------------
    # Optional dependency checks
    # ------------------------------------------------------------------

    @api_telemetry
    def check_can_use_arrow_resultset(self) -> None:
        check_dependency(pyarrow)

    @api_telemetry
    def check_can_use_pandas(self) -> None:
        check_dependency(pandas)
