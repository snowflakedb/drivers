"""
Base cursor class and supporting decorators.

This module defines the abstract base cursor class (``SnowflakeCursorBase``)
and its associated helpers: ``FetchMode``, type aliases, and decorator
functions for precondition checks.
"""

from __future__ import annotations

import abc
import ctypes
import enum
import functools
import logging

from collections.abc import Iterator, Sequence
from typing import TYPE_CHECKING, Any, Callable, TypeVar, cast, overload

from .._internal.arrow_stream_utils import (
    collect_arrow_table,
    create_row_iterator,
    create_table_iterator,
)
from .._internal.binding_converters import (
    ClientSideBindingConverter,
    JsonBindingConverter,
    ParamStyle,
)
from .._internal.config_utils import create_config_setting
from .._internal.decorators import pep249
from .._internal.errorcode import ER_CURSOR_IS_CLOSED, ER_INVALID_VALUE
from .._internal.errorhandler import ErrorHandlerMixin
from .._internal.extras import check_dependency, pandas, pyarrow, requires_dependency
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    BinaryDataPtr,
    ConnectionAbortQueryRequest,
    ConnectionGetQueryResultRequest,
    ConnectionGetResultSetRequest,
    ExecuteQueryResponse,
    PrepareResult,
    QueryBindings,
    ResultChunk,
    ResultSetResponse,
    StatementExecuteAsyncRequest,
    StatementExecuteQueryRequest,
    StatementGetResultSetRequest,
    StatementHandle,
    StatementPrepareRequest,
    StatementResultChunksRequest,
    StatementSetOptionsRequest,
)
from .._internal.statement_utils import create_statement
from ..errors import Error, ErrorValue, InterfaceError, NotSupportedError, ProgrammingError
from ..result_batch import ResultBatch
from ._query_result import _QueryResult
from ._query_result_waiter import QueryResultWaiter
from ._result_metadata import QueryResultStats, ResultMetadata


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from .._internal.arrow_stream_iterator import ArrowStreamIterator
    from ..connection import Connection

logger = logging.getLogger(__name__)

Row = tuple[Any, ...]
DictRow = dict[str, Any]


F = TypeVar("F", bound=Callable[..., Any])
T = TypeVar("T", bound=Sequence[Any])


class FetchMode(enum.Enum):
    """Distinguishes row-by-row fetching from Arrow/Pandas fetching.

    Once a cursor begins consuming results with one mode, switching to
    the other is disallowed until a new ``execute()`` resets state.
    """

    ROW = "row"
    ARROW = "arrow"


def _requires_open(func: F) -> F:
    @functools.wraps(func)
    def wrapper(self: SnowflakeCursorBase, *args: Any, **kwargs: Any) -> Any:
        if self.is_closed():
            raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _requires_open_cursor_not_connection(func: F) -> F:
    """Guard that only checks ``self._closed``, ignoring the connection state.

    Unlike ``_requires_open`` (which delegates to ``is_closed()`` and therefore
    also rejects cursors whose *connection* has been closed), this decorator
    deliberately skips the connection check.  This preserves backward
    compatibility with the old driver, where fetch methods on a cursor with
    already-buffered results still worked after ``connection.close()``.
    """

    @functools.wraps(func)
    def wrapper(self: SnowflakeCursorBase, *args: Any, **kwargs: Any) -> Any:
        if self._closed:
            raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _with_prefetch_hook(func: F) -> F:
    """Invoke the cursor's prefetch hook (if set) before entering the wrapped method."""

    @functools.wraps(func)
    def wrapper(self: SnowflakeCursorBase, *args: Any, **kwargs: Any) -> Any:
        if self._prefetch_hook is not None:
            self._prefetch_hook()
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _requires_fetch_mode(mode: FetchMode) -> Callable[[F], F]:
    """Validate and lock the cursor's fetch mode before entering the wrapped method."""

    def decorator(func: F) -> F:
        @functools.wraps(func)
        def wrapper(self: SnowflakeCursorBase, *args: Any, **kwargs: Any) -> Any:
            if self._fetch_mode and self._fetch_mode != mode:
                if mode == FetchMode.ARROW:
                    raise ProgrammingError(msg="Cannot use arrow/pandas fetch methods after row-by-row fetching")
                elif mode == FetchMode.ROW:
                    raise ProgrammingError(msg="Cannot use row-by-row fetch methods after arrow/pandas fetching")
                else:
                    raise ProgrammingError(msg=f"Unexpected fetch mode: {mode}")
            self._fetch_mode = mode

            return func(self, *args, **kwargs)

        return cast(F, wrapper)

    return decorator


class SnowflakeCursorBase(ErrorHandlerMixin, abc.ABC):
    """
    Base cursor class for database operations (PEP 249).

    This is the abstract base for all cursor types, equivalent to
    ``SnowflakeCursorBase`` in the old connector. Concrete subclasses
    must override :pyattr:`_use_dict_result` and :pymeth:`fetchone`.
    """

    def __init__(self, connection: Connection) -> None:
        """
        Initialize a new cursor object.

        Args:
            connection: Connection object that created this cursor
        """
        # -- Core cursor state (identity, lifecycle, error handling) --
        self._connection: Connection = connection
        self._closed: bool = False
        self._messages: list[tuple[type[Exception], ErrorValue]] = []
        self._errorhandler: Callable[..., None] = Error.default_errorhandler

        # -- PEP 249 cursor configuration (persists for cursor lifetime) --
        self._arraysize: int = 1

        # -- Query result state (replaced on execute, mutated on reset/consume) --
        self._query_result: _QueryResult = _QueryResult()

        # Cursor navigation position — mutable to avoid allocation per fetchone
        self._rownumber: int = -1

        # -- Active iteration state (cleared on reset) --
        self._result_chunks: list[ResultChunk] | None = None
        self._iterator: ArrowStreamIterator | None = None
        self._fetch_mode: FetchMode | None = None

        # -- Multi-statement state (cleared on execute/reset) --
        self._multi_statement_query_ids: list[str] = []
        self._multi_statement_current_index: int = 0
        self._multi_statement_parent_qid: str | None = None

        # -- Statement parameters (persists until explicitly changed) --
        self._statement_parameters: dict[str, Any] = {}

        # Keep binding data reference to prevent garbage collection while Rust uses it
        self._binding_data: None | bytes = None
        # Deferred result loading (set by get_results_from_sfqid, invoked on first fetch)
        self._prefetch_hook: Callable[[], None] | None = None

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
    @pep249
    def connection(self) -> Connection:
        """The :class:`Connection` object that created this cursor."""
        return self._connection

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
    # Result format control
    # ------------------------------------------------------------------

    @property
    @abc.abstractmethod
    def _use_dict_result(self) -> bool:
        """Whether fetch methods return dicts instead of tuples."""

    # ------------------------------------------------------------------
    # Execution
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
        return self._multi_statement_parent_qid

    @property
    def multi_statement_savedIds(self) -> list[str]:
        """
        Read-only attribute containing child query IDs for multi-statement queries.

        Returns:
            list[str]: List of child query IDs (empty list for single statements).
        """
        return self._multi_statement_query_ids

    @overload
    def callproc(self, procname: str) -> tuple: ...

    @overload
    def callproc(self, procname: str, args: T) -> T: ...

    @pep249
    @_requires_open
    def callproc(self, procname: str, args: Any = None) -> Any:
        """Call a stored procedure.

        Args:
            procname: The stored procedure to be called.
            args: Parameters to be passed into the stored procedure.
                  ``None`` is treated as no arguments.

        Returns:
            The input parameters.
        """
        if args is None:
            args = ()
        if isinstance(args, (str, bytes)):
            raise TypeError(f"callproc args must be a sequence (e.g. list or tuple), not {type(args).__name__}")
        if not isinstance(args, Sequence):
            raise TypeError(f"callproc args must be a sequence (e.g. list or tuple), not {type(args).__name__}")
        command = f"CALL {procname}({self._connection.paramstyle.placeholders(len(args))})"
        self.execute(command, args)
        return args

    @_requires_open
    def set_statement_parameter(self, key: str, value: Any) -> None:
        """Set a statement-level parameter (e.g., MULTI_STATEMENT_COUNT).

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
        # Store in cursor for application in _execute
        self._statement_parameters[key] = value

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
        self, operation: str, parameters: Sequence[Any] | dict[str, Any] | None
    ) -> tuple[str, QueryBindings | None]:
        """Prepare query and bindings based on paramstyle.

        Args:
            operation: SQL statement
            parameters: Parameters to bind (sequence or dict)

        Returns:
            Tuple of (query string, QueryBindings or None)

        Raises:
            ProgrammingError: If dict parameters used with server-side binding
        """
        if parameters is None:
            return operation, None

        paramstyle = self._connection.paramstyle

        if paramstyle.is_client_side():
            # format paramstyle only supports positional params (%s), not named params
            if paramstyle == ParamStyle.FORMAT and isinstance(parameters, dict):
                raise ProgrammingError(
                    msg="Dict parameters not supported with format paramstyle. "
                    "Use pyformat paramstyle for named parameters, or use a sequence.",
                    errno=ER_INVALID_VALUE,
                )
            # Client-side binding: interpolate parameters into SQL string
            query = ClientSideBindingConverter.interpolate_query(operation, parameters)
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

    @pep249
    @_requires_open
    def execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        num_statements: int | None = None,
        **kwargs: Any,
    ) -> SnowflakeCursorBase:
        """
        Execute a database operation (query or command).
        Resets the cursor state before the execution.

        Args:
            operation (str): SQL statement to execute
            parameters (sequence or dict): Parameters for the operation.
                For qmark/numeric paramstyle: sequence of values
                For pyformat paramstyle: sequence (%s) or dict (%(name)s)
                For format paramstyle: sequence (%s)
            num_statements (int, optional): Number of statements in a multistatement query.
        """
        if num_statements is not None:
            # TODO Create a global known parameters registry
            self.set_statement_parameter("MULTI_STATEMENT_COUNT", num_statements)

        self.reset()
        return self._execute(operation, parameters, _is_put_get, **kwargs)

    def _format_query_for_log(self, query: str) -> str:
        return self._connection._format_query_for_log(query)

    def _execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        **kwargs: Any,
    ) -> SnowflakeCursorBase:
        """Execute query logic."""
        if logger.isEnabledFor(logging.DEBUG):
            logger.debug("query: [%s]", self._format_query_for_log(operation))

        query, bindings = self._prepare_query(operation, parameters)

        with create_statement(self.connection, query) as stmt_handle:
            # Apply statement parameters if set
            if self._statement_parameters:
                self._apply_statement_parameters(stmt_handle)

            response = self._execute_query(stmt_handle, bindings)

            # Dispatch based on response type
            if response.HasField("multi"):
                self._handle_multi_statement_response(response, stmt_handle, query)
            else:
                self._handle_single_statement_response(response, stmt_handle, query)

        self._rownumber = -1  # reset the rownumber (rownumber is not reset in reset() for backward compatibility)
        return self

    def _fetch_result_chunk_metadata(
        self, stmt_handle: StatementHandle, query_id: str | None = None
    ) -> list[ResultChunk] | None:
        """Retrieve chunk metadata for a specific query ID while the statement handle is alive.

        For single-statement queries, query_id should be None (server infers from statement handle).
        For multi-statement queries, query_id must specify which child query's chunks to return.
        """
        try:
            # Only include query_id in request if explicitly provided (for multistatement)
            if query_id is not None:
                logger.debug(f"Fetching chunk metadata WITH query_id={query_id}")
                request = StatementResultChunksRequest(stmt_handle=stmt_handle, query_id=query_id)
            else:
                logger.debug("Fetching chunk metadata WITHOUT query_id")
                request = StatementResultChunksRequest(stmt_handle=stmt_handle)
            response = self._connection.db_api.statement_result_chunks(request)
            if response.HasField("result"):
                return list(response.result.chunks)
            logger.warning("No result field in response")
            return None
        except Exception:
            logger.warning(f"Failed to fetch result chunk metadata for query_id={query_id}", exc_info=True)
            return None

    def _execute_query(self, stmt_handle: StatementHandle, bindings: QueryBindings | None) -> ExecuteQueryResponse:
        """Execute query and return ExecuteQueryResponse (single or multi)."""
        try:
            request = StatementExecuteQueryRequest(stmt_handle=stmt_handle, bindings=bindings)
            return self._connection.db_api.statement_execute_query(request)
        except ProgrammingError as exc:
            self._query_result = _QueryResult.from_programming_error(exc)
            raise

    def _fetch_result_set(self, stmt_handle: StatementHandle, query_id: str) -> ResultSetResponse:
        """Fetch a result set by query ID via StatementGetResultSet RPC.

        Args:
            stmt_handle: Active statement handle.
            query_id: Query ID to fetch.

        Returns:
            ResultSetResponse containing descriptor and arrow stream.

        Raises:
            ProgrammingError: If the fetch fails.
        """
        try:
            request = StatementGetResultSetRequest(
                stmt_handle=stmt_handle,
                query_id=query_id,
            )
            return self._connection.db_api.statement_get_result_set(request)
        except Exception as exc:
            if isinstance(exc, ProgrammingError):
                raise
            raise ProgrammingError(msg=f"Failed to fetch result set for query_id={query_id}: {exc}") from exc

    def _handle_single_statement_response(
        self, response: ExecuteQueryResponse, stmt_handle: StatementHandle, query: str
    ) -> None:
        """Handle single-statement execution response."""
        # Clear any previous multistatement state
        self._multi_statement_query_ids = []
        self._multi_statement_current_index = 0
        self._multi_statement_parent_qid = None

        descriptor = response.single

        # Fetch the result set (metadata + arrow stream)
        result_set_response = self._fetch_result_set(stmt_handle, descriptor.query_id)
        self._query_result = _QueryResult._from_result_set_response(result_set_response, descriptor, query)
        # For single-statement, pass the query_id to chunks request
        self._result_chunks = self._fetch_result_chunk_metadata(stmt_handle, query_id=descriptor.query_id)

    def _handle_multi_statement_response(
        self, response: ExecuteQueryResponse, stmt_handle: StatementHandle, query: str
    ) -> None:
        """Handle multi-statement execution response."""
        multi_result = response.multi

        # Store parent query ID and child IDs
        parent = multi_result.parent
        self._multi_statement_parent_qid = parent.query_id if parent.query_id else None

        # Extract query IDs from multi-statement result
        self._multi_statement_query_ids = list(multi_result.query_ids)
        self._multi_statement_current_index = 0

        # Edge case: empty multi-statement result
        if not self._multi_statement_query_ids:
            self._query_result = _QueryResult(query=query)
            self._result_chunks = None
            self._multi_statement_parent_qid = None  # Clear parent QID for consistency
            return

        # Immediately fetch and apply the first child result
        first_qid = self._multi_statement_query_ids[0]
        result_set_response = self._fetch_result_set(stmt_handle, first_qid)
        descriptor = result_set_response.result_descriptor

        # For first result in multistatement, pass the full query
        self._query_result = _QueryResult._from_result_set_response(result_set_response, descriptor, query)
        self._result_chunks = self._fetch_result_chunk_metadata(stmt_handle, first_qid)
        self._multi_statement_current_index = 1

    def _apply_statement_parameters(self, stmt_handle: StatementHandle) -> None:
        """Apply stored statement parameters to the statement handle via SetOptions RPC."""
        if not self._statement_parameters:
            return

        # Build options map with ConfigSetting values
        options = {}
        for key, value in self._statement_parameters.items():
            try:
                setting = create_config_setting(value)
            except TypeError as err:
                raise TypeError(f"Cannot set parameter '{key}': {err}") from err
            if setting is not None:
                options[key] = setting

        # Send single RPC with all options
        request = StatementSetOptionsRequest(stmt_handle=stmt_handle, options=options)
        self._connection.db_api.statement_set_options(request)

    def _prepare(self, stmt_handle: StatementHandle) -> PrepareResult | None:
        try:
            request = StatementPrepareRequest(stmt_handle=stmt_handle)
            return self._connection.db_api.statement_prepare(request).result
        except ProgrammingError as exc:
            self._query_result = _QueryResult.from_programming_error(exc)
            raise

    @pep249
    @_requires_open
    def executemany(self, operation: str, seq_of_parameters: Sequence[Sequence[Any] | dict[str, Any]]) -> None:
        """
        Execute a database operation repeatedly for each element in seq_of_parameters.

        For qmark/numeric paramstyles, uses array binding to execute all parameter
        sets in a single request. For pyformat/format paramstyles, executes each
        row individually with client-side interpolation.

        Args:
            operation (str): SQL statement (typically INSERT, UPDATE, or DELETE)
            seq_of_parameters (sequence): Sequence of parameter sequences or dicts

        Raises:
            InterfaceError: If parameter sequences have inconsistent lengths
        """
        if not seq_of_parameters:
            return  # Empty sequence - no-op per PEP 249

        paramstyle = self._connection.paramstyle
        first_params = seq_of_parameters[0]

        # Execute individually for:
        # - Client-side binding (pyformat/format)
        # - Dict parameters (server-side doesn't support named binding)
        if paramstyle.is_client_side() or isinstance(first_params, dict):
            self.reset()
            total_rowcount = 0
            unknown_rowcount = False
            for params in seq_of_parameters:
                self._execute(operation, params)  # no reset between calls
                rc = self._query_result.rowcount
                if rc is None or rc == -1:
                    unknown_rowcount = True
                elif not unknown_rowcount:
                    total_rowcount += rc
            # Per PEP 249, -1 indicates that the number of rows is unknown,
            # but for backward compatibility it's set to None.
            self._query_result.rowcount = None if unknown_rowcount else total_rowcount
            return

        # Server-side binding: validate and use array binding
        # Dict params were handled above; only sequences reach here.
        rows = cast(Sequence[Sequence[Any]], seq_of_parameters)

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
        num_columns = first_len
        transposed = [[row[col_idx] for row in rows] for col_idx in range(num_columns)]

        # Execute using array binding (existing path handles list values)
        self.execute(operation, transposed)

    @_requires_open
    def describe(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> list[ResultMetadata] | None:
        """Obtain the schema of the result without executing the query.

        This method prepares the query on the server with describeOnly=true to obtain
        column metadata without actually executing the query or returning data rows.

        Args:
            operation: SQL statement to describe
            parameters: Parameters for the SQL statement (same as execute())
            **kwargs: Additional keyword arguments (for future compatibility)

        Returns:
            List of ResultMetadata tuples describing result columns, or None if the
            statement produces no result set (e.g., INSERT, UPDATE, DELETE, DDL).

        Side effects:
            - Updates cursor.description with the column metadata
        """
        self.reset()
        query, bindings = self._prepare_query(operation, parameters)

        prepare_result: PrepareResult | None = None
        with create_statement(self.connection, query) as stmt_handle:
            prepare_result = self._prepare(stmt_handle)

        self._query_result = _QueryResult.from_prepare_result(prepare_result)

        if self._query_result.description:
            self._rownumber = -1

        return self._query_result.description

    # ------------------------------------------------------------------
    # Fetch – shared implementation
    # ------------------------------------------------------------------

    @_requires_open_cursor_not_connection
    @_with_prefetch_hook
    @_requires_fetch_mode(FetchMode.ROW)
    def _fetchone(self) -> Row | DictRow | None:
        """Fetch the next row internally.

        Return a dict if ``_use_dict_result`` is True, otherwise a tuple.
        Concrete subclasses expose this through a type-safe ``fetchone``.
        """
        if not self._iterator:
            self._iterator = self._create_row_iterator()
        try:
            row: Row | DictRow = next(self._iterator)
            self._rownumber += 1
            return row
        except StopIteration:
            return None

    @pep249
    @abc.abstractmethod
    def fetchone(self) -> Row | DictRow | None:
        """Fetch the next row of a query result set."""

    @pep249
    @_requires_open_cursor_not_connection
    @_requires_fetch_mode(FetchMode.ROW)
    def fetchmany(self, size: int | None = None) -> list[Any]:
        """
        Fetch the next set of rows of a query result.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            sequence: List of rows

        Raises:
            ProgrammingError: If the number of rows is not zero or positive number
        """
        if size is None:
            size = self.arraysize

        if size < 0:
            raise ProgrammingError(
                msg=f"The number of rows is not zero or positive number: {size}", errno=ER_INVALID_VALUE
            )

        if size == 0:
            return []

        if not self._iterator:
            self._iterator = self._create_row_iterator()
        rows = self._iterator.fetch_many(size)
        self._rownumber += len(rows)
        return rows

    @pep249
    @_requires_open_cursor_not_connection
    @_with_prefetch_hook
    @_requires_fetch_mode(FetchMode.ROW)
    def fetchall(self) -> list[Any]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            sequence: List of all remaining rows
        """
        if not self._iterator:
            self._iterator = self._create_row_iterator()
        rows = self._iterator.fetch_all()
        self._rownumber += len(rows)
        return rows

    # ------------------------------------------------------------------
    # Iterator protocol
    # ------------------------------------------------------------------

    def _create_row_iterator(self) -> ArrowStreamIterator:
        return create_row_iterator(
            stream_ptr=self._query_result.consume_stream(),
            use_dict_result=self._use_dict_result,
            use_numpy=self._connection._numpy,
        )

    @pep249
    def __iter__(self) -> SnowflakeCursorBase:
        """
        Return the cursor itself as an iterator.

        Returns:
            SnowflakeCursorBase: Self
        """
        return self

    def __next__(self) -> Row | DictRow:
        """
        Fetch the next row from the currently executed statement.

        Returns:
            sequence: Next row

        Raises:
            StopIteration: When no more rows are available
        """
        row = self.fetchone()
        if row is None:
            raise StopIteration
        return row

    @pep249
    def next(self) -> Row | DictRow:
        """Python 2 compatibility method."""
        return self.__next__()

    # ------------------------------------------------------------------
    # PEP 249 optional / no-op methods
    # ------------------------------------------------------------------

    @pep249
    @_requires_open
    def nextset(self) -> SnowflakeCursorBase | None:
        """
        Skip to the next available result set, discarding remaining rows from current set.

        This method is used for multi-statement queries where a single execute() produces
        multiple result sets. Call nextset() to advance to the next query's results.

        Returns:
            SnowflakeCursorBase: Self if next set is available.
            None: If no more result sets are available.

        Raises:
            InterfaceError: If cursor is closed.

        Example:
            cursor.set_statement_parameter("MULTI_STATEMENT_COUNT", 3)
            cursor.execute("SELECT 1; SELECT 2; SELECT 3")
            print(cursor.fetchone())  # (1,)
            cursor.nextset()
            print(cursor.fetchone())  # (2,)
            cursor.nextset()
            print(cursor.fetchone())  # (3,)
            result = cursor.nextset()  # None - no more results
        """
        # Check if there are more child results to fetch
        if self._multi_statement_current_index >= len(self._multi_statement_query_ids):
            return None

        # Save multistatement state before reset (reset clears it)
        query_ids = self._multi_statement_query_ids
        current_idx = self._multi_statement_current_index
        parent_qid = self._multi_statement_parent_qid

        # Reset cursor state for new result set
        self.reset()

        # Restore multistatement state
        self._multi_statement_query_ids = query_ids
        self._multi_statement_current_index = current_idx + 1
        self._multi_statement_parent_qid = parent_qid

        # Fetch the next child result
        query_id = query_ids[current_idx]

        # Use connection-level GetResultSet since statement handle is no longer available
        request = ConnectionGetResultSetRequest(
            conn_handle=self._connection.conn_handle,
            query_id=query_id,
        )
        result_set_response = self._connection.db_api.connection_get_result_set(request)
        descriptor = result_set_response.result_descriptor

        self._query_result = _QueryResult._from_result_set_response(result_set_response, descriptor)
        self._rownumber = -1
        # Note: chunk metadata not available for child results without statement handle
        self._result_chunks = None

        return self

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

    @pep249
    def scroll(self, value: int, mode: str = "relative") -> None:
        """Scroll the cursor in the result set."""
        raise NotSupportedError("scroll is not supported")

    # ------------------------------------------------------------------
    # Context manager
    # ------------------------------------------------------------------

    def __enter__(self) -> SnowflakeCursorBase:
        """
        Enter the runtime context for the cursor.

        Returns:
            SnowflakeCursorBase: Self
        """
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit the runtime context for the cursor."""
        self.close()

    def is_closed(self) -> bool:
        """
        Check if the cursor is closed.

        Returns:
            bool: True if closed, False otherwise
        """
        return self._closed or self._connection.is_closed()

    @_requires_open_cursor_not_connection
    def reset(self, closing: bool = False) -> None:
        """Reset the result set.

        Frees heavy result data (arrow streams) while for backward compatibility
        preserving metadata that the old driver also keeps across resets:
        ``description``, ``rownumber``, ``sfqid``, ``query``, and ``sqlstate``.

        Multi-statement state is also cleared: query IDs, current index, and parent query ID.

        Also clears the ``messages`` list so that errors from previous
        operations do not leak into the next one.

        Args:
            closing: If True, do not reset rowcount,
                     see: SNOW-647539: Do not erase the rowcount information when closing the cursor.
                     If False, reset rowcount to None.
        """
        del self._messages[:]
        self._query_result.reset(closing=closing)
        self._result_chunks = None
        self._iterator = None
        self._fetch_mode = None
        self._binding_data = None
        self._prefetch_hook = None
        # Clear multistatement state
        self._multi_statement_query_ids = []
        self._multi_statement_current_index = 0
        self._multi_statement_parent_qid = None

    @pep249
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

    # ------------------------------------------------------------------
    # Session parameter accessors
    # ------------------------------------------------------------------

    @property
    def timestamp_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_OUTPUT_FORMAT`` parameter value."""
        return self._connection._get_session_parameter("TIMESTAMP_OUTPUT_FORMAT")

    @property
    def timestamp_ltz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_LTZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return self._connection._get_session_parameter("TIMESTAMP_LTZ_OUTPUT_FORMAT") or self.timestamp_output_format

    @property
    def timestamp_tz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_TZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return self._connection._get_session_parameter("TIMESTAMP_TZ_OUTPUT_FORMAT") or self.timestamp_output_format

    @property
    def timestamp_ntz_output_format(self) -> str | None:
        """The session's ``TIMESTAMP_NTZ_OUTPUT_FORMAT`` parameter value.

        Falls back to :pyattr:`timestamp_output_format` when not set explicitly.
        """
        return self._connection._get_session_parameter("TIMESTAMP_NTZ_OUTPUT_FORMAT") or self.timestamp_output_format

    @property
    def date_output_format(self) -> str | None:
        """The session's ``DATE_OUTPUT_FORMAT`` parameter value."""
        return self._connection._get_session_parameter("DATE_OUTPUT_FORMAT")

    @property
    def time_output_format(self) -> str | None:
        """The session's ``TIME_OUTPUT_FORMAT`` parameter value."""
        return self._connection._get_session_parameter("TIME_OUTPUT_FORMAT")

    @property
    def timezone(self) -> str | None:
        """The session's ``TIMEZONE`` parameter value."""
        return self._connection._get_session_parameter("TIMEZONE")

    @property
    def binary_output_format(self) -> str | None:
        """The session's ``BINARY_OUTPUT_FORMAT`` parameter value (``HEX`` or ``BASE64``)."""
        return self._connection._get_session_parameter("BINARY_OUTPUT_FORMAT")

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

    @property
    def _errorhandler_connection(self) -> Connection:
        return self._connection

    @property
    def _errorhandler_cursor(self) -> SnowflakeCursorBase:
        return self

    # ------------------------------------------------------------------
    # Fetch – Arrow / Pandas
    # ------------------------------------------------------------------

    @requires_dependency(pyarrow)
    @_requires_open
    @_with_prefetch_hook
    @_requires_fetch_mode(FetchMode.ARROW)
    def fetch_arrow_batches(
        self,
        force_microsecond_precision: bool = False,
    ) -> Iterator[Table]:
        """Fetch Arrow Tables in batches."""
        iterator = create_table_iterator(
            stream_ptr=self._query_result.consume_stream(),
            number_to_decimal=self._connection.arrow_number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        )
        for batch in iterator:
            yield pyarrow.Table.from_batches([batch])

    @requires_dependency(pyarrow)
    @_requires_open
    @_with_prefetch_hook
    @_requires_fetch_mode(FetchMode.ARROW)
    def fetch_arrow_all(
        self,
        force_return_table: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table | None:
        """Fetch all results as a single Arrow Table."""
        iterator = create_table_iterator(
            stream_ptr=self._query_result.consume_stream(),
            number_to_decimal=self._connection.arrow_number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        )
        return collect_arrow_table(
            table_iterator=iterator,
            columns_metadata=self._query_result.description,
            force_return_table=force_return_table,
        )

    @requires_dependency(pandas)
    @_requires_open
    def fetch_pandas_batches(self, **kwargs: Any) -> Iterator[DataFrame]:
        """Fetch Pandas DataFrames in batches."""
        for table in self.fetch_arrow_batches(**kwargs):
            yield table.to_pandas()

    @requires_dependency(pandas)
    @_requires_open
    def fetch_pandas_all(self, **kwargs: Any) -> DataFrame:
        """Fetch all results as a single Pandas DataFrame."""
        table: Table = self.fetch_arrow_all(force_return_table=True, **kwargs)
        return table.to_pandas()

    def check_can_use_arrow_resultset(self) -> None:
        check_dependency(pyarrow)

    def check_can_use_pandas(self) -> None:
        check_dependency(pandas)

    # ------------------------------------------------------------------
    # Distributed fetch
    # ------------------------------------------------------------------

    @_requires_open
    def get_result_batches(self) -> list[ResultBatch] | None:
        """Get the previously executed query's ResultBatches if available."""
        return ResultBatch.from_chunks(self._result_chunks, self._query_result.description, self._connection)

    # ------------------------------------------------------------------
    # Async query support
    # ------------------------------------------------------------------

    @_requires_open
    def query_result(self, qid: str) -> SnowflakeCursorBase:
        """Fetch the result of a previously executed query by its Snowflake Query ID.

        Resets the cursor and populates it with the results from the specified
        query, making them available through the standard fetch methods
        (fetchone, fetchall, fetch_arrow_all, etc.).

        Args:
            qid: Snowflake Query ID (sfqid) of the previously executed query.

        Returns:
            This cursor instance, now populated with the query results.

        Raises:
            ProgrammingError: If the query ID is invalid, the query is still
                running, or the results are no longer available.
        """
        self.reset()

        request = ConnectionGetQueryResultRequest(
            conn_handle=self._connection.conn_handle,
            query_id=qid,
        )
        response = self._connection.db_api.connection_get_query_result(request)

        # Handle single or multi-statement response
        if response.HasField("multi"):
            # For async, we only support fetching the first result of multi-statement
            multi_result = response.multi
            if multi_result.query_ids:
                first_qid = multi_result.query_ids[0]
                result_request = ConnectionGetResultSetRequest(
                    conn_handle=self._connection.conn_handle,
                    query_id=first_qid,
                )
                result_set = self._connection.db_api.connection_get_result_set(result_request)
                self._query_result = _QueryResult._from_result_set_response(result_set)
            else:
                self._query_result = _QueryResult()
        else:
            # Single statement
            descriptor = response.single
            result_request = ConnectionGetResultSetRequest(
                conn_handle=self._connection.conn_handle,
                query_id=descriptor.query_id,
            )
            result_set = self._connection.db_api.connection_get_result_set(result_request)
            self._query_result = _QueryResult._from_result_set_response(result_set, descriptor)

        self._rownumber = -1

        return self

    @_requires_open
    def get_results_from_sfqid(self, sfqid: str) -> None:
        """Get results from a previously executed query.

        Polls query status until completion, then loads results lazily
        on first fetch call.

        Before the first fetch triggers the prefetch hook, result-dependent
        cursor attributes and methods such as ``description``, ``rowcount``,
        and ``fetch*`` are not populated and MUST NOT be relied upon.

        Args:
            sfqid: Snowflake Query ID of the target query.

        Raises:
            ProgrammingError: If the query has already failed at call time,
                or if it reaches a terminal error status while being polled
                for completion.
            DatabaseError: If the server stops returning status information
                while polling for query completion.
        """
        self.reset()
        self.connection.get_query_status_throw_if_error(sfqid)
        self._query_result.sfqid = sfqid
        waiter = QueryResultWaiter(self._connection, sfqid)

        def prefetch_hook() -> None:
            waiter.wait()
            self._prefetch_hook = None
            self.query_result(sfqid)

        self._prefetch_hook = prefetch_hook

    @_requires_open
    def execute_async(
        self,
        command: str,
        params: Sequence[Any] | dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> dict[str, str | None]:
        """Submit a query for async execution and return immediately with the query ID.

        This is the first step in the async query lifecycle::

            # 1. Submit the query
            result = cursor.execute_async("SELECT ...")
            query_id = result["queryId"]

            # 2. Poll until complete
            status = connection.get_query_status(query_id)

            # 3. Retrieve results
            cursor.get_results_from_sfqid(query_id)

        Args:
            command: SQL statement to execute.
            params: Parameters for the operation (sequence or dict).
            **kwargs: Unused, accepted for backward compatibility.

        Returns:
            dict with a ``queryId`` key containing the Snowflake Query ID.
        """
        # TODO: deprecate returning the dict, return just the sfqid itself
        self.reset()
        return self._execute_async(command, params)

    def _execute_async(self, command: str, params: Sequence[Any] | dict[str, Any] | None) -> dict[str, str | None]:
        query, bindings = self._prepare_query(command, params)

        response = None
        with create_statement(self._connection, query) as stmt_handle:
            request = StatementExecuteAsyncRequest(stmt_handle=stmt_handle, bindings=bindings)
            response = self._connection.db_api.statement_execute_async(request)

        query_id = (response.query_id if response.query_id else None) if response else None
        self._query_result = _QueryResult(sfqid=query_id)

        return {"queryId": query_id}

    @_requires_open
    def abort_query(self, qid: str) -> bool:
        """Abort a running query."""
        request = ConnectionAbortQueryRequest(
            conn_handle=self._connection.conn_handle,
            query_id=qid,
        )
        response = self._connection.db_api.connection_abort_query(request)
        return response.success
