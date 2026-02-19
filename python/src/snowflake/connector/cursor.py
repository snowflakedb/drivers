"""
PEP 249 Database API 2.0 Cursor Objects

This module defines the cursor classes as specified in PEP 249.

Hierarchy:
    SnowflakeCursorBase
    ├── SnowflakeCursor  — returns tuple rows
    └── DictCursor       — returns dict rows
"""

from __future__ import annotations

import abc
import ctypes

from collections.abc import Iterator, Sequence
from typing import TYPE_CHECKING, Any, NamedTuple, cast

from ._internal.arrow_context import ArrowConverterContext
from ._internal.arrow_stream_iterator import ArrowStreamIterator
from ._internal.binding_converters import (
    ClientSideBindingConverter,
    JsonBindingConverter,
    ParamStyle,
)
from ._internal.protobuf_gen.database_driver_v1_pb2 import (
    BinaryDataPtr,
    ExecuteResult,
    QueryBindings,
    StatementExecuteQueryRequest,
    StatementNewRequest,
    StatementSetSqlQueryRequest,
)
from ._internal.type_codes import get_type_code
from .errors import InterfaceError, NotSupportedError, ProgrammingError


if TYPE_CHECKING:
    from .connection import Connection

Row = tuple[Any, ...]
DictRow = dict[str, Any]


class ResultMetadata(NamedTuple):
    """PEP 249 column description entry.

    Each item in ``Cursor.description`` is a ``ResultMetadata`` instance.
    Being a :class:`~typing.NamedTuple` it is fully tuple-compatible as
    required by the spec, while also providing named attribute access.
    """

    name: str
    type_code: int
    display_size: int | None
    internal_size: int | None
    precision: int | None
    scale: int | None
    is_nullable: bool | None

    @classmethod
    def from_column(cls, col: Any) -> ResultMetadata:
        """Create a ``ResultMetadata`` from a protobuf ``ColumnMetadata``."""
        type_code = get_type_code(col.type)

        display_size = (
            col.length if col.HasField("length") and col.type.upper() in ("TEXT", "VARCHAR", "CHAR", "STRING") else None
        )
        internal_size = col.byte_length if col.HasField("byte_length") else None
        precision = col.precision if col.HasField("precision") else None
        scale = col.scale if col.HasField("scale") else None

        return cls(
            name=col.name,
            type_code=type_code,
            display_size=display_size,
            internal_size=internal_size,
            precision=precision,
            scale=scale,
            is_nullable=col.nullable,
        )


# Backward compatibility alias
ResultMetadataV2 = ResultMetadata


class SnowflakeCursorBase(abc.ABC):
    """
    Base cursor class for database operations (PEP 249).

    This is the abstract base for all cursor types, equivalent to
    ``SnowflakeCursorBase`` in the old connector. Concrete subclasses
    must override :pyattr:`_use_dict_result` and :pymeth:`fetchone`.
    """

    # Class attribute for arraysize
    arraysize = 1

    def __init__(self, connection: Connection) -> None:
        """
        Initialize a new cursor object.

        Args:
            connection: Connection object that created this cursor
        """
        self.connection = connection
        self._description: list[ResultMetadata] | None = None
        self._rowcount: int | None = None
        self.arraysize = 1  # Instance attribute overrides class attribute
        self._closed = False
        # Streaming state for Arrow results
        self._reader = None
        self._current_batch = None
        self._current_row_in_batch = 0
        self.execute_result: ExecuteResult | None = None
        self._iterator: Iterator[Row] | None = None
        # Query bindings - keep binding data reference to prevent garbage collection while Rust uses it
        self._binding_data: None | bytes = None

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
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
    def rowcount(self) -> int | None:
        """
        Read-only attribute specifying the number of rows that the last
        .execute*() produced or affected.

        Returns:
            int: Number of rows affected, or None if not determined
        """
        return self._rowcount

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
    def sfqid(self) -> str | None:
        """
        Read-only attribute containing the Snowflake Query ID for the last executed query.

        Returns:
            str | None: Snowflake Query ID (UUID format), or None if no query has been executed
        """
        if self.execute_result is None:
            return None
        return self.execute_result.query_id if self.execute_result.query_id else None

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

    def close(self) -> None:
        """Close the cursor now (rather than whenever __del__ is called)."""
        self._closed = True

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

        paramstyle = self.connection.paramstyle  # Always returns ParamStyle enum

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

    def execute(self, operation: str, parameters: Sequence[Any] | dict[str, Any] | None = None) -> SnowflakeCursorBase:
        """
        Execute a database operation (query or command).

        Args:
            operation (str): SQL statement to execute
            parameters (sequence or dict): Parameters for the operation.
                For qmark/numeric paramstyle: sequence of values
                For pyformat paramstyle: sequence (%s) or dict (%(name)s)
                For format paramstyle: sequence (%s)
        """
        query, bindings = self._prepare_query(operation, parameters)

        stmt_handle = self.connection.db_api.statement_new(
            StatementNewRequest(conn_handle=self.connection.conn_handle)
        ).stmt_handle
        self.connection.db_api.statement_set_sql_query(
            StatementSetSqlQueryRequest(stmt_handle=stmt_handle, query=query)
        )

        request = StatementExecuteQueryRequest(stmt_handle=stmt_handle, bindings=bindings)

        self.execute_result = self.connection.db_api.statement_execute_query(request).result

        # Reset streaming state for a new result
        self._binding_data = None
        self._iterator = None

        # Populate description and rowcount
        self._populate_description()
        self._populate_rowcount()
        return self

    def _populate_rowcount(self) -> None:
        if self.execute_result:
            self._rowcount = self.execute_result.rows_affected
        else:
            self._rowcount = None

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

        paramstyle = self.connection.paramstyle
        first_params = seq_of_parameters[0]

        # Execute individually for:
        # - Client-side binding (pyformat/format)
        # - Dict parameters (server-side doesn't support named binding)
        if paramstyle.is_client_side() or isinstance(first_params, dict):
            total_rowcount = 0
            unknown_rowcount = False
            for params in seq_of_parameters:
                self.execute(operation, params)
                rc = self._rowcount
                if rc is None or rc == -1:
                    unknown_rowcount = True
                elif not unknown_rowcount:
                    total_rowcount += rc
            # Per PEP 249, -1 indicates that the number of rows is unknown
            self._rowcount = -1 if unknown_rowcount else total_rowcount
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

    # ------------------------------------------------------------------
    # Arrow stream helpers
    # ------------------------------------------------------------------

    def _get_stream_ptr(self) -> int:
        """Get the ArrowArrayStream pointer from execute result.

        Returns:
            int: The ArrowArrayStream pointer as an integer

        Raises:
            RuntimeError: If execute_result is invalid or stream pointer is null
        """
        if self.execute_result is None:
            raise RuntimeError("No query has been executed")

        if not hasattr(self.execute_result, "stream") or self.execute_result.stream is None:
            raise RuntimeError("Execute result does not contain a valid stream")

        if not hasattr(self.execute_result.stream, "value") or self.execute_result.stream.value is None:
            raise RuntimeError("Stream does not contain a valid pointer value")

        stream_value = self.execute_result.stream.value
        if len(stream_value) != 8:
            raise RuntimeError(f"Stream pointer value has wrong length: {len(stream_value)} (expected 8)")

        stream_ptr = int.from_bytes(stream_value, byteorder="little", signed=False)

        if stream_ptr == 0:
            raise RuntimeError("Stream pointer is null")

        return stream_ptr

    def _populate_description(self) -> None:
        """Populate cursor description from execute result column metadata."""
        if self.execute_result is None:
            self._description = None
            return

        columns = self.execute_result.columns
        if not columns:
            self._description = None
            return

        self._description = [ResultMetadata.from_column(col) for col in columns]

    def _get_iterator(self) -> ArrowStreamIterator:
        stream_ptr = self._get_stream_ptr()
        arrow_context = ArrowConverterContext()
        return ArrowStreamIterator(
            stream_ptr,
            arrow_context,
            use_dict_result=self._use_dict_result,
            # TODO: SNOW-2997786, temporarily hardcoded
            use_numpy=False,
        )

    # ------------------------------------------------------------------
    # Fetch – shared implementation
    # ------------------------------------------------------------------

    def _fetchone(self) -> Row | DictRow | None:
        """Fetch the next row internally.

        Return a dict if ``_use_dict_result`` is True, otherwise a tuple.
        Concrete subclasses expose this through a type-safe ``fetchone``.
        """
        if self._iterator is None:
            self._iterator = self._get_iterator()
        try:
            return next(self._iterator)
        except StopIteration:
            return None

    @abc.abstractmethod
    def fetchone(self) -> Row | DictRow | None:
        """Fetch the next row of a query result set."""

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

    def fetchall(self) -> list[Any]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            sequence: List of all remaining rows
        """
        if self._iterator is None:
            self._iterator = self._get_iterator()
        return list(self._iterator)

    # ------------------------------------------------------------------
    # PEP 249 optional / no-op methods
    # ------------------------------------------------------------------

    def nextset(self) -> None:
        """
        Skip to the next available set, discarding any remaining rows from current set.

        Returns:
            bool: True if next set is available, False/None otherwise

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("nextset is not implemented")

    def setinputsizes(self, sizes: Sequence[Any]) -> None:
        """Not supported."""
        return None

    def setoutputsize(self, size: int, column: int | None = None) -> None:
        """Not supported."""
        return None

    # ------------------------------------------------------------------
    # Iterator protocol
    # ------------------------------------------------------------------

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

    # Python 2 compatibility
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


# ======================================================================
# Concrete cursor classes
# ======================================================================


class SnowflakeCursor(SnowflakeCursorBase):
    """Cursor returning results as tuples (default).

    This is the standard cursor returned by ``connection.cursor()``.
    """

    @property
    def _use_dict_result(self) -> bool:
        return False

    def fetchone(self) -> Row | None:
        """
        Fetch the next row of a query result set.

        Returns:
            tuple: Next row, or None when no more data is available
        """
        row = self._fetchone()
        if not (row is None or isinstance(row, tuple)):
            raise TypeError(f"fetchone got unexpected result: {row}")
        return row

    def fetchmany(self, size: int | None = None) -> list[Row]:
        """
        Fetch the next set of rows of a query result.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            list[tuple]: List of rows as tuples
        """
        return super().fetchmany(size)

    def fetchall(self) -> list[Row]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            list[tuple]: List of all remaining rows as tuples
        """
        return super().fetchall()


class DictCursor(SnowflakeCursorBase):
    """Cursor returning results as dictionaries with column names as keys.

    Usage::

        with connection.cursor(DictCursor) as cur:
            cur.execute("SELECT 1 AS id, 'hello' AS name")
            row = cur.fetchone()
            # row == {"ID": 1, "NAME": "hello"}
    """

    @property
    def _use_dict_result(self) -> bool:
        return True

    def fetchone(self) -> DictRow | None:
        """
        Fetch the next row of a query result set as a dictionary.

        Returns:
            dict: Next row as a dictionary with column names as keys,
                  or None when no more data is available
        """
        row = self._fetchone()
        if not (row is None or isinstance(row, dict)):
            raise TypeError(f"fetchone got unexpected result: {row}")
        return row

    def fetchmany(self, size: int | None = None) -> list[DictRow]:
        """
        Fetch the next set of rows as dictionaries.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            list[dict]: List of rows as dictionaries
        """
        return super().fetchmany(size)

    def fetchall(self) -> list[DictRow]:
        """
        Fetch all (remaining) rows as dictionaries.

        Returns:
            list[dict]: List of all remaining rows as dictionaries
        """
        return super().fetchall()


__all__ = ["SnowflakeCursor", "DictCursor"]
