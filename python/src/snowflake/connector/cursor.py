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

from collections.abc import Iterator, Sequence
from typing import TYPE_CHECKING, Any

from ._internal.arrow_context import ArrowConverterContext
from ._internal.arrow_stream_iterator import ArrowStreamIterator  # type: ignore[import-not-found]
from ._internal.protobuf_gen.database_driver_v1_pb2 import (  # type: ignore[attr-defined]
    StatementExecuteQueryRequest,
    StatementNewRequest,
    StatementSetSqlQueryRequest,
)
from .errors import NotSupportedError, ProgrammingError


if TYPE_CHECKING:
    from .connection import Connection

Row = tuple[Any, ...]
DictRow = dict[str, Any]


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
        self.description = None
        self.rowcount = -1
        self.arraysize = 1  # Instance attribute overrides class attribute
        self._closed = False
        # Streaming state for Arrow results
        self._reader = None
        self._current_batch = None
        self._current_row_in_batch = 0
        self.execute_result: Any = None
        self._iterator: Iterator[Row] | Iterator[DictRow] | None = None

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

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

    def execute(self, operation: str, parameters: Sequence[Any] | dict[str, Any] | None = None) -> SnowflakeCursorBase:
        """
        Execute a database operation (query or command).

        Args:
            operation (str): SQL statement to execute
            parameters (sequence or mapping): Parameters for the operation
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
        return self

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
