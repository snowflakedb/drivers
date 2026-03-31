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

from collections.abc import Iterator, Sequence
from typing import TYPE_CHECKING, Any, Callable, TypeVar, cast

from snowflake.connector._internal.errorcode import ER_CONNECTION_IS_CLOSED, ER_CURSOR_IS_CLOSED

from .._internal.arrow_context import ArrowConverterContext
from .._internal.arrow_stream_iterator import ArrowStreamIterator, ArrowStreamTableIterator
from .._internal.binding_converters import (
    ClientSideBindingConverter,
    JsonBindingConverter,
    ParamStyle,
)
from .._internal.decorators import pep249
from .._internal.extras import check_dependency, pandas, pyarrow, requires_dependency
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    BinaryDataPtr,
    ConnectionGetQueryResultRequest,
    ExecuteResult,
    PrepareResult,
    QueryBindings,
    StatementExecuteQueryRequest,
    StatementHandle,
    StatementPrepareRequest,
)
from .._internal.query_utils import (
    create_statement,
    extract_rowcount,
    extract_sqlstate,
    get_stream_ptr,
    release_arrow_stream,
)
from .._internal.type_codes import FIXED
from ..errors import InterfaceError, NotSupportedError, ProgrammingError
from ._result_metadata import QueryResultStats, ResultMetadata


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Schema, Table

    from ..connection import Connection

Row = tuple[Any, ...]
DictRow = dict[str, Any]

F = TypeVar("F", bound=Callable[..., Any])


class FetchMode(enum.Enum):
    """Distinguishes row-by-row fetching from Arrow/Pandas fetching.

    Once a cursor begins consuming results with one mode, switching to
    the other is disallowed until a new ``execute()`` resets state.
    """

    ROW = "row"
    ARROW = "arrow"


def _requires_not_closed(func: F) -> F:

    @functools.wraps(func)
    def wrapper(self: SnowflakeCursorBase, *args: Any, **kwargs: Any) -> Any:
        if self._closed:
            raise InterfaceError("Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)

        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _requires_open_connection(func: F) -> F:
    """Raise InterfaceError if the underlying connection is closed."""

    @functools.wraps(func)
    def wrapper(self: SnowflakeCursorBase, *args: Any, **kwargs: Any) -> Any:
        if self.connection.is_closed():
            raise InterfaceError("Connection is closed.", errno=ER_CONNECTION_IS_CLOSED)

        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _requires_fetch_mode(mode: FetchMode) -> Callable[[F], F]:
    """Validate and lock the cursor's fetch mode before entering the wrapped method."""

    def decorator(func: F) -> F:
        @functools.wraps(func)
        def wrapper(self: SnowflakeCursorBase, *args: Any, **kwargs: Any) -> Any:
            if self._fetch_mode and self._fetch_mode != mode:
                if mode == FetchMode.ARROW:
                    raise ProgrammingError("Cannot use arrow/pandas fetch methods after row-by-row fetching")
                elif mode == FetchMode.ROW:
                    raise ProgrammingError("Cannot use row-by-row fetch methods after arrow/pandas fetching")
                else:
                    raise ProgrammingError(f"Unexpected fetch mode: {mode}")
            self._fetch_mode = mode

            return func(self, *args, **kwargs)

        return cast(F, wrapper)

    return decorator


class SnowflakeCursorBase(abc.ABC):
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
        self._connection = connection
        self._description: list[ResultMetadata] | None = None
        self._rowcount: int | None = None
        self._arraysize: int = 1
        self._sqlstate: str | None = None
        self._sfqid: str | None = None
        self._query: str | None = None
        self._closed = False
        # Streaming state for Arrow results
        self.execute_result: ExecuteResult | None = None
        self._iterator: Iterator[Row] | None = None
        self._fetch_mode: FetchMode | None = None
        # Query bindings - keep binding data reference to prevent garbage collection while Rust uses it
        self._binding_data: None | bytes = None
        self._messages: list[tuple[type[Exception], dict[str, str | bool]]] = []
        self._rownumber: int = -1
        self._errorhandler: Callable

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
        return self._description

    @property
    @pep249
    def rowcount(self) -> int | None:
        """
        Read-only attribute specifying the number of rows that the last
        .execute*() produced or affected.

        Returns:
            int: Number of rows affected, or None if not determined
        """
        return self._rowcount

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
    def messages(self) -> list[tuple[type[Exception], dict[str, str | bool]]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], dict[str, str | bool]]]) -> None:
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
        Read-only attribute containing the SQL text of the last executed query.

        Returns:
            str | None: The SQL query string, or None if no query has been executed
        """
        return self._query

    @property
    def sfqid(self) -> str | None:
        """
        Read-only attribute containing the Snowflake Query ID for the last executed query.

        Returns:
            str | None: Snowflake Query ID (UUID format), or None if no query has been executed
        """
        return self._sfqid

    @property
    def stats(self) -> QueryResultStats | None:
        """Returns detailed row-level statistics for DML operations."""
        if self.execute_result is None or not self.execute_result.HasField("stats"):
            return QueryResultStats()
        return QueryResultStats.from_query_stats(self.execute_result.stats)

    @pep249
    def callproc(self, procname: str, parameters: Sequence[Any] | None = None) -> Sequence[Any]:
        """
        Call a stored database procedure with the given name.

        Args:
            procname (str): Name of the procedure to call
            parameters (sequence): Input parameters for the procedure

        Returns:
            sequence: The result of the procedure call

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("callproc is not implemented")

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

        paramstyle = self._connection.paramstyle  # Always returns ParamStyle enum

        if paramstyle.is_client_side():
            # format paramstyle only supports positional params (%s), not named params
            if paramstyle == ParamStyle.FORMAT and isinstance(parameters, dict):
                raise ProgrammingError(
                    "Dict parameters not supported with format paramstyle. "
                    "Use pyformat paramstyle for named parameters, or use a sequence."
                )
            # Client-side binding: interpolate parameters into SQL string
            query = ClientSideBindingConverter.interpolate_query(operation, parameters)
            return query, None
        else:
            # Server-side binding: qmark or numeric
            if isinstance(parameters, dict):
                raise ProgrammingError(
                    "Named parameters (dict) not supported with qmark/numeric paramstyle. "
                    "Use pyformat paramstyle for named parameters."
                )
            bindings = self._build_query_bindings(parameters)
            return operation, bindings

    @pep249
    @_requires_not_closed
    @_requires_open_connection
    def execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
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
        """
        self.reset()
        return self._execute(operation, parameters, _is_put_get, **kwargs)

    def _execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        **kwargs: Any,
    ) -> SnowflakeCursorBase:
        """Execute query logic."""
        query, bindings = self._prepare_query(operation, parameters)

        result: ExecuteResult | None = None
        with create_statement(self.connection, query) as stmt_handle:
            result = self._execute_query(stmt_handle, bindings)

        self._populate_state_from_execute_result(result)

        return self

    def _execute_query(self, stmt_handle: StatementHandle, bindings: QueryBindings | None) -> ExecuteResult:
        try:
            request = StatementExecuteQueryRequest(stmt_handle=stmt_handle, bindings=bindings)
            return self._connection.db_api.statement_execute_query(request).result
        except ProgrammingError as exc:
            self._sqlstate = exc.sqlstate or None
            raise

    def _prepare(self, stmt_handle: StatementHandle) -> PrepareResult:
        try:
            request = StatementPrepareRequest(stmt_handle=stmt_handle)
            return self._connection.db_api.statement_prepare(request).result
        except ProgrammingError as exc:
            self._sqlstate = exc.sqlstate or None
            raise

    def _populate_state_from_execute_result(self, result: ExecuteResult | None) -> None:
        self._description = ResultMetadata.create_description(result)
        self._rowcount = extract_rowcount(result)
        self._sqlstate = extract_sqlstate(result)
        self._sfqid = (result.query_id if result.query_id else None) if result else None
        self._query = (result.query if result.query else None) if result else None
        # reset the rownumber (rownumber is not reset in reset() for backward compatibility)
        self._rownumber = -1
        # save execute result (holds arrow stream data needed for fetching)
        self.execute_result = result

    @pep249
    @_requires_not_closed
    @_requires_open_connection
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
            ProgrammingError: If parameter sequences have inconsistent lengths
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
                rc = self._rowcount
                if rc is None or rc == -1:
                    unknown_rowcount = True
                elif not unknown_rowcount:
                    total_rowcount += rc
            # Per PEP 249, -1 indicates that the number of rows is unknown
            # but for backward compatibility we set it to None
            self._rowcount = None if unknown_rowcount else total_rowcount
            return

        # Server-side binding: validate and use array binding
        # Dict params were handled above; only sequences reach here.
        rows = cast(Sequence[Sequence[Any]], seq_of_parameters)

        # Error code 251007 (ER_INVALID_VALUE) matches reference driver behavior
        first_len = len(first_params)
        for params in rows:
            if len(params) != first_len:
                raise InterfaceError(
                    f"251007: Bulk data size don't match. expected: {first_len}, "
                    f"got: {len(params)}, command: {operation}"
                )

        # Transpose from row-major to column-major format
        # Input:  [(row1_col1, row1_col2), (row2_col1, row2_col2), ...]
        # Output: [[row1_col1, row2_col1, ...], [row1_col2, row2_col2, ...]]
        num_columns = first_len
        transposed = [[row[col_idx] for row in rows] for col_idx in range(num_columns)]

        # Execute using array binding (existing path handles list values)
        self.execute(operation, transposed)

    @_requires_not_closed
    @_requires_open_connection
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

        result: PrepareResult | None = None
        with create_statement(self.connection, query) as stmt_handle:
            result = self._prepare(stmt_handle)

        stream_ptr = get_stream_ptr(result)
        release_arrow_stream(stream_ptr)

        result_metadata = ResultMetadata.create_description(result)
        if result_metadata:
            self._description = result_metadata  # shallow copy
            self._rowcount = 0
            self._sqlstate = None
            # reset the rownumber (rownumber is not reset in reset() for backward compatibility)
            self._rownumber = -1

        return result_metadata

    # ------------------------------------------------------------------
    # Arrow stream helpers
    # ------------------------------------------------------------------

    def _get_iterator(self) -> ArrowStreamIterator:
        stream_ptr = get_stream_ptr(self.execute_result)
        arrow_context = ArrowConverterContext()
        return ArrowStreamIterator(
            stream_ptr,
            arrow_context,
            use_dict_result=self._use_dict_result,
            # TODO: SNOW-2997786, temporarily hardcoded
            use_numpy=False,
        )

    def _get_table_iterator(
        self,
        force_microsecond_precision: bool = False,
    ) -> ArrowStreamTableIterator:
        stream_ptr = get_stream_ptr(self.execute_result)
        arrow_context = ArrowConverterContext()
        return ArrowStreamTableIterator(
            stream_ptr,
            arrow_context,
            number_to_decimal=self._connection.arrow_number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        )

    # ------------------------------------------------------------------
    # Fetch – shared implementation
    # ------------------------------------------------------------------

    @_requires_not_closed
    @_requires_fetch_mode(FetchMode.ROW)
    def _fetchone(self) -> Row | DictRow | None:
        """Fetch the next row internally.

        Return a dict if ``_use_dict_result`` is True, otherwise a tuple.
        Concrete subclasses expose this through a type-safe ``fetchone``.
        """
        if self._iterator is None:
            self._iterator = self._get_iterator()
        try:
            row = next(self._iterator)
            self._rownumber += 1
            return row
        except StopIteration:
            return None

    @pep249
    @abc.abstractmethod
    def fetchone(self) -> Row | DictRow | None:
        """Fetch the next row of a query result set."""

    @pep249
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
            raise ProgrammingError(f"The number of rows is not zero or positive number: {size}")

        ret = []
        while size > 0:
            row = self.fetchone()
            if row is None:
                break
            ret.append(row)
            size -= 1

        return ret

    @pep249
    @_requires_not_closed
    @_requires_fetch_mode(FetchMode.ROW)
    def fetchall(self) -> list[Any]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            sequence: List of all remaining rows
        """
        if self._iterator is None:
            self._iterator = self._get_iterator()
        rows = list(self._iterator)
        self._rownumber += len(rows)
        return rows

    # ------------------------------------------------------------------
    # PEP 249 optional / no-op methods
    # ------------------------------------------------------------------

    @pep249
    def nextset(self) -> None:
        """
        Skip to the next available set, discarding any remaining rows from current set.

        Returns:
            bool: True if next set is available, False/None otherwise

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("nextset is not implemented")

    @pep249
    def setinputsizes(self, sizes: Sequence[Any]) -> None:
        """Not supported."""
        return None

    @pep249
    def setoutputsize(self, size: int, column: int | None = None) -> None:
        """Not supported."""
        return None

    # ------------------------------------------------------------------
    # Iterator protocol
    # ------------------------------------------------------------------

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
        return self._closed

    @property
    @pep249
    def rownumber(self) -> int | None:
        """The current 0-based index of the cursor in the result set, or ``None`` if indeterminate."""
        return self._rownumber if self._rownumber >= 0 else None

    @property
    def sqlstate(self) -> str | None:
        """The SQLSTATE code of the last executed operation."""
        return self._sqlstate

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
    def is_file_transfer(self) -> bool:
        """Whether the last executed command was a PUT or GET file transfer."""
        raise NotImplementedError("is_file_transfer is not yet implemented")

    @property
    @pep249
    def lastrowid(self) -> None:
        """Snowflake does not support lastrowid; returns None per PEP 249."""
        return None

    def execute_async(
        self,
        command: str,
        params: Sequence[Any] | dict[str, Any] | None = None,
        timeout: int | None = None,
        **kwargs: Any,
    ) -> dict[str, Any]:
        """Execute a query asynchronously without waiting for results."""
        raise NotImplementedError("execute_async is not yet implemented")

    @pep249
    def scroll(self, value: int, mode: str = "relative") -> None:
        """Scroll the cursor in the result set."""
        raise NotSupportedError("scroll is not supported")

    def reset(self, closing: bool = False) -> None:
        """Reset the result set.

        Frees heavy result data (arrow streams) while for backward compatibility
        preserving metadata that the old driver also keeps across resets:
        ``description``, ``rownumber``, ``sfqid``, ``query``, and ``sqlstate``.

        Args:
            closing: If True, do not reset rowcount,
                     see: SNOW-647539: Do not erase the rowcount information when closing the cursor.
                     If False, reset rowcount to None.
        """
        if not closing:
            self._rowcount = None
        self.execute_result = None
        self._iterator = None
        self._fetch_mode = None
        self._binding_data = None

    @_requires_not_closed
    @requires_dependency(pyarrow)
    @_requires_fetch_mode(FetchMode.ARROW)
    def fetch_arrow_batches(
        self,
        force_microsecond_precision: bool = False,
    ) -> Iterator[Table]:
        """Fetch Arrow Tables in batches."""
        iterator = self._get_table_iterator(
            force_microsecond_precision=force_microsecond_precision,
        )
        for batch in iterator:
            yield pyarrow.Table.from_batches([batch])

    @_requires_not_closed
    @requires_dependency(pyarrow)
    @_requires_fetch_mode(FetchMode.ARROW)
    def fetch_arrow_all(
        self,
        force_return_table: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table | None:
        """Fetch all results as a single Arrow Table."""
        iterator = self._get_table_iterator(
            force_microsecond_precision=force_microsecond_precision,
        )
        batches = list(iterator)
        if not batches:
            if force_return_table:
                schema = iterator.get_converted_schema()
                normalized_schema = self._normalize_fixed_column_types(schema)
                return normalized_schema.empty_table()
            return None
        return pyarrow.Table.from_batches(batches)

    def _normalize_fixed_column_types(self, schema: Schema) -> Schema:
        """Rewrite FIXED columns in an empty-result schema to int64 for backward compatibility.
        When the result set has zero rows, the core chooses a narrower integer type
        (i.e. int8) for NUMBER or float64 for SCALED NUMBER.
        The old driver always exposed int64 in this case.
        We use cursor.description (which is populated from query metadata) to identify FIXED columns and cast them.
        """
        if not self._description:
            return schema

        new_fields = []
        changed = False
        for field, metadata in zip(schema, self._description):
            if metadata.type_code == FIXED and field.type != pyarrow.int64():
                new_fields.append(field.with_type(pyarrow.int64()))
                changed = True
            else:
                new_fields.append(field)
        return pyarrow.schema(new_fields) if changed else schema

    @_requires_not_closed
    @requires_dependency(pandas)
    def fetch_pandas_batches(self, **kwargs: Any) -> Iterator[DataFrame]:
        """Fetch Pandas DataFrames in batches."""
        for table in self.fetch_arrow_batches(**kwargs):
            yield table.to_pandas()

    @_requires_not_closed
    @requires_dependency(pandas)
    def fetch_pandas_all(self, **kwargs: Any) -> DataFrame:
        """Fetch all results as a single Pandas DataFrame."""
        table: Table = self.fetch_arrow_all(force_return_table=True, **kwargs)
        return table.to_pandas()

    def check_can_use_arrow_resultset(self) -> None:
        check_dependency(pyarrow)

    def check_can_use_pandas(self) -> None:
        check_dependency(pandas)

    @_requires_not_closed
    @_requires_open_connection
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

        self._populate_state_from_execute_result(response.result)

        return self

    def abort_query(self, qid: str) -> bool:
        """Abort a running query."""
        raise NotImplementedError("abort_query is not yet implemented")

    def get_results_from_sfqid(self, sfqid: str) -> None:
        """Get results from a previously executed async query."""
        raise NotImplementedError("get_results_from_sfqid is not yet implemented")

    def get_result_batches(self) -> list[Any] | None:
        """Get the previously executed query's ResultBatches if available."""
        raise NotImplementedError("get_result_batches is not yet implemented")
