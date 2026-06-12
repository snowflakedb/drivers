"""Shared cursor base mixin for sync and async cursor implementations."""

from __future__ import annotations

import ctypes

from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING, Any, cast

from ...errors import Error, ErrorValue, InterfaceError, ProgrammingError
from ..binding_converters import (
    ClientSideBindingConverter,
    JsonBindingConverter,
    ParamStyle,
)
from ..config_utils import create_config_setting
from ..decorators import pep249
from ..errorcode import ER_INVALID_VALUE
from ..errorhandler import ErrorHandlerMixin
from ..extras import check_dependency, pandas, pyarrow
from ..protobuf_gen.database_driver_v1_pb2 import BinaryDataPtr, ConfigSetting, QueryBindings
from .query_result import MultiStatementQueryResultState, QueryResult
from .result_metadata import QueryResultStats, ResultMetadata


if TYPE_CHECKING:
    from ..._async.connection import AsyncConnection
    from ...connection import Connection


class CursorBaseMixin(ErrorHandlerMixin):
    """Zero-I/O cursor members shared by sync and async base cursor classes."""

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
    def description(self) -> list[ResultMetadata] | None:
        """
        Read-only attribute describing the result columns of a query.

        Returns a sequence of 7-item tuples, each containing:
        - name: Column name (str)
        - type_code: Integer type code (int)
        - display_size: Display size in characters (int | None)
        - internal_size: Internal size in bytes (int | None)
        - precision: Precision for numeric types (int | None)
        - scale: Scale for numeric types (int | None)
        - null_ok: True if column can contain NULLs (bool | None)

        Returns None if no query has been executed or if the query didn't produce a result set.
        """
        return self._query_result.description

    @property
    @pep249
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
    def arraysize(self) -> int:
        """Number of rows to fetch at a time with .fetchmany(). Defaults to 1."""
        return self._arraysize

    @arraysize.setter
    def arraysize(self, value: int) -> None:
        self._arraysize = int(value)

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    # ------------------------------------------------------------------
    # Execution metadata
    # ------------------------------------------------------------------

    @property
    def query(self) -> str | None:
        """
        Read-only attribute containing the SQL text of the last executed or described query.

        Returns:
            str | None: The SQL query string, or None if no query has been executed or described
        """
        return self._query_result.query

    @property
    def sfqid(self) -> str | None:
        """
        Read-only attribute containing the Snowflake Query ID for the last executed or described query.

        Returns:
            str | None: Snowflake Query ID (UUID format), or None if no query has been executed or described
        """
        return self._query_result.sfqid

    @property
    def stats(self) -> QueryResultStats:
        """Returns detailed row-level statistics for DML operations."""
        return self._query_result.stats

    @property
    @pep249
    def rownumber(self) -> int | None:
        """The current 0-based index of the cursor in the result set, or ``None`` if indeterminate."""
        return self._rownumber if self._rownumber >= 0 else None

    @property
    def sqlstate(self) -> str | None:
        """The SQLSTATE code of the last executed operation."""
        return self._query_result.sqlstate

    @property
    def multi_statement_parent_sfqid(self) -> str | None:
        """
        Read-only attribute containing the parent Snowflake Query ID for multi-statement queries.

        Returns:
            str | None: Parent query ID for multi-statement, or None for single statements.
        """
        return self._multi_statement.parent_qid if self._multi_statement else None

    @property
    def multi_statement_savedIds(self) -> list[str]:
        """
        Read-only attribute containing child query IDs for multi-statement queries.

        Returns:
            list[str]: List of child query IDs (empty list for single statements).
        """
        return self._multi_statement.child_query_ids if self._multi_statement else []

    @property
    def is_file_transfer(self) -> bool:
        """Whether the last executed command was a PUT or GET file transfer."""
        raise NotImplementedError("is_file_transfer is not yet implemented")

    def _build_query_bindings(self, parameters: Sequence[Any]) -> QueryBindings | None:
        """Serialize parameters and build a QueryBindings protobuf message.

        Converts Python parameter values to JSON via JsonBindingConverter, then
        wraps the result in a zero-copy BinaryDataPtr so the Rust core can read
        the JSON directly from Python memory.

        The encoded bytes are stored on ``self._binding_data`` to prevent
        garbage collection while Rust holds the pointer.

        Returns:
            QueryBindings with the serialized JSON, or None if parameters
            serialize to nothing (e.g. empty list).
        """
        json_str, length = JsonBindingConverter.serialize_parameters(parameters)
        if json_str is None:
            return None

        # Convert string to bytes and keep a reference to prevent garbage
        # collection while Rust uses the underlying buffer.
        json_bytes = json_str.encode("utf-8")
        self._binding_data = json_bytes

        # Get memory address of the bytes buffer (no-copy scheme)
        ptr_value = ctypes.cast(ctypes.c_char_p(json_bytes), ctypes.c_void_p).value
        if ptr_value is None:
            raise RuntimeError("Failed to obtain memory pointer for binding data")

        # Convert pointer to 8-byte little-endian representation
        ptr_bytes = ptr_value.to_bytes(8, byteorder="little", signed=False)

        binary_data_ptr = BinaryDataPtr(
            value=ptr_bytes,  # 8-byte pointer value
            length=length,
        )
        return QueryBindings(json=binary_data_ptr)

    def _prepare_query(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None,
        _force_qmark_paramstyle: bool = False,
    ) -> tuple[str, QueryBindings | None]:
        """Prepare query and bindings based on paramstyle.

        Args:
            _force_qmark_paramstyle: If True, treat the call as qmark
                regardless of the connection's configured paramstyle. Used by
                callers (e.g. snowflake.core) that emit ``?`` placeholders
                while connections may default to pyformat.

        Raises:
            ProgrammingError: If dict parameters used with server-side binding
        """
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
            bindings = self._build_query_bindings(parameters)
            return operation, bindings

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

    def _build_statement_parameter_options(self) -> dict[str, ConfigSetting]:
        """Build ConfigSetting options from stored statement parameters."""
        options: dict[str, ConfigSetting] = {}
        for key, value in self._statement_parameters.items():
            try:
                setting = create_config_setting(value)
            except TypeError as err:
                raise TypeError(f"Cannot set parameter '{key}': {err}") from err
            if setting is not None:
                options[key] = setting
        return options

    @pep249
    def setinputsizes(self, sizes: Sequence[Any]) -> None:
        """Not supported."""
        return None

    @pep249
    def setoutputsize(self, size: int, column: int | None = None) -> None:
        """Not supported."""
        return None

    @property
    @pep249
    def lastrowid(self) -> None:
        """Snowflake does not support lastrowid; returns None per PEP 249."""
        return None

    # ------------------------------------------------------------------
    # Session parameter accessors
    # ------------------------------------------------------------------

    @property
    def timestamp_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_OUTPUT_FORMAT`` parameter value."""
        return self._connection._session_parameters["TIMESTAMP_OUTPUT_FORMAT"]

    @property
    def timestamp_ltz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_LTZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return self._connection._session_parameters["TIMESTAMP_LTZ_OUTPUT_FORMAT"] or self.timestamp_output_format

    @property
    def timestamp_tz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_TZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return self._connection._session_parameters["TIMESTAMP_TZ_OUTPUT_FORMAT"] or self.timestamp_output_format

    @property
    def timestamp_ntz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_NTZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return self._connection._session_parameters["TIMESTAMP_NTZ_OUTPUT_FORMAT"] or self.timestamp_output_format

    @property
    def date_output_format(self) -> str | None:
        """The session's ``DATE_OUTPUT_FORMAT`` parameter value."""
        return self._connection._session_parameters["DATE_OUTPUT_FORMAT"]

    @property
    def time_output_format(self) -> str | None:
        """The session's ``TIME_OUTPUT_FORMAT`` parameter value."""
        return self._connection._session_parameters["TIME_OUTPUT_FORMAT"]

    @property
    def timezone(self) -> str | None:
        """The session's ``TIMEZONE`` parameter value."""
        return self._connection._session_parameters["TIMEZONE"]

    @property
    def binary_output_format(self) -> str | None:
        """The session's ``BINARY_OUTPUT_FORMAT`` parameter value (``HEX`` or ``BASE64``)."""
        return self._connection._session_parameters["BINARY_OUTPUT_FORMAT"]

    # ------------------------------------------------------------------
    # Error handling
    # ------------------------------------------------------------------

    @property
    @pep249
    def errorhandler(self) -> Callable:
        """PEP 249 error handler for this cursor."""
        return self._errorhandler

    @errorhandler.setter
    def errorhandler(self, value: Callable | None) -> None:
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

    # ------------------------------------------------------------------
    # Optional dependency checks
    # ------------------------------------------------------------------

    def check_can_use_arrow_resultset(self) -> None:
        check_dependency(pyarrow)

    def check_can_use_pandas(self) -> None:
        check_dependency(pandas)
