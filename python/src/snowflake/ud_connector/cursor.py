"""
PEP 249 Database API 2.0 Cursor Objects

This module defines the Cursor class as specified in PEP 249.
"""

from __future__ import annotations

from collections.abc import Iterator, Sequence
from typing import TYPE_CHECKING, Any

from ._internal.arrow_context import ArrowConverterContext
from ._internal.arrow_stream_iterator import ArrowStreamIterator  # type: ignore[import-untyped]
from ._internal.protobuf_gen.database_driver_v1_pb2 import (  # type: ignore[attr-defined]
    StatementExecuteQueryRequest,
    StatementNewRequest,
    StatementSetSqlQueryRequest,
)
from .exceptions import NotSupportedError


if TYPE_CHECKING:
    from .connection import Connection

Row = tuple[Any, ...]


class Cursor:
    """
    Cursor objects represent a database cursor, which is used to manage the context
    of a fetch operation.
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
        self.description = None
        self.rowcount = -1
        self.arraysize = 1  # Instance attribute overrides class attribute
        self._closed = False
        # Streaming state for Arrow results
        self._reader = None
        self._current_batch = None
        self._current_row_in_batch = 0
        self.execute_result: Any = None
        self._iterator: Iterator[Row] | None = None
        self.execute_result = None

    @property
    def description(self) -> Any:
        """
        Read-only attribute describing the result columns of a query.

        Returns:
            tuple: Sequence of 7-item tuples describing each result column:
                   (name, type_code, display_size, internal_size, precision, scale, null_ok)
        """
        return self._description

    @description.setter
    def description(self, value: Any) -> None:
        self._description = value

    @property
    def rowcount(self) -> int:
        """
        Read-only attribute specifying the number of rows that the last
        .execute*() produced or affected.

        Returns:
            int: Number of rows affected, or -1 if not determined
        """
        return self._rowcount

    @rowcount.setter
    def rowcount(self, value: int) -> None:
        self._rowcount = value

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

    def execute(self, operation: str, parameters: Sequence[Any] | dict[str, Any] | None = None) -> None:
        """
        Execute a database operation (query or command).

        Args:
            operation (str): SQL statement to execute
            parameters (sequence or mapping): Parameters for the operation

        Raises:
            NotSupportedError: If not implemented
        """
        stmt_handle = self.connection.db_api.statement_new(
            StatementNewRequest(conn_handle=self.connection.conn_handle)
        ).stmt_handle
        self.connection.db_api.statement_set_sql_query(
            StatementSetSqlQueryRequest(stmt_handle=stmt_handle, query=operation)
        )
        self.execute_result = self.connection.db_api.statement_execute_query(
            StatementExecuteQueryRequest(stmt_handle=stmt_handle)
        ).result
        # Reset streaming state for a new result
        self._iterator = None

    def executemany(self, operation: str, seq_of_parameters: Sequence[Sequence[Any]]) -> None:
        """
        Execute a database operation repeatedly for each element in seq_of_parameters.

        Args:
            operation (str): SQL statement to execute
            seq_of_parameters (sequence): Sequence of parameter sequences

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("executemany is not implemented")

    def _get_stream_ptr(self) -> int:
        """Get the ArrowArrayStream pointer from execute result."""
        stream_ptr = int.from_bytes(self.execute_result.stream.value, byteorder="little", signed=False)
        return stream_ptr

    def _ensure_iterator(self) -> None:
        if self._iterator is None:
            stream_ptr = self._get_stream_ptr()
            arrow_context = ArrowConverterContext()
            self._iterator = ArrowStreamIterator(
                stream_ptr,
                arrow_context,
                # TODO: SNOW-2997742, SNOW-2997786, temporarily hardcoded
                use_dict_result=False,
                use_numpy=False,
            )

    def fetchone(self) -> Row | None:
        """
        Fetch the next row of a query result set.

        Returns:
            sequence: Next row, or None when no more data is available

        Raises:
            NotSupportedError: If not implemented
        """
        self._ensure_iterator()
        assert self._iterator is not None
        try:
            return next(self._iterator)
        except StopIteration:
            return None

    def fetchmany(self, size: int | None = None) -> list[Row]:
        """
        Fetch the next set of rows of a query result.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            sequence: List of rows

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("fetchmany is not implemented")

    def fetchall(self) -> list[Row]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            sequence: List of all remaining rows

        Raises:
            NotSupportedError: If not implemented
        """
        self._ensure_iterator()
        assert self._iterator is not None
        return list(self._iterator)

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
        """
        Predefine memory areas for the operation parameters.

        Args:
            sizes (sequence): Sequence of type objects or integers
        """
        # This method is optional and can be implemented as a no-op
        pass

    def setoutputsize(self, size: int, column: int | None = None) -> None:
        """
        Set a column buffer size for fetches of large columns.

        Args:
            size (int): Buffer size
            column (int): Column index (optional)
        """
        # This method is optional and can be implemented as a no-op
        pass

    def __iter__(self) -> Cursor:
        """
        Return the cursor itself as an iterator.

        Returns:
            Cursor: Self
        """
        return self

    def __next__(self) -> Row:
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
    def next(self) -> Row:
        """Python 2 compatibility method."""
        return self.__next__()

    def __enter__(self) -> Cursor:
        """
        Enter the runtime context for the cursor.

        Returns:
            Cursor: Self
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
