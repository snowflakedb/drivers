"""
PEP 249 Database API 2.0 Cursor Objects

This module defines the Cursor class as specified in PEP 249.
"""

from __future__ import annotations

from .exceptions import NotSupportedError, ProgrammingError

from ._internal.protobuf_gen.database_driver_v1_pb2 import (
    StatementNewRequest,
    StatementSetSqlQueryRequest,
    StatementExecuteQueryRequest,
)
from ._arrow_stream_iterator import ArrowStreamIterator
from .arrow_context import ArrowConverterContext


class Cursor:
    """
    Cursor objects represent a database cursor, which is used to manage the context
    of a fetch operation.
    """

    def __init__(self, connection, use_dict_result=False, use_numpy=False):
        """
        Initialize a new cursor object.

        Args:
            connection: Connection object that created this cursor
            use_dict_result: If True, return dicts instead of tuples
            use_numpy: If True, use numpy types for numeric data
        """
        self.connection = connection
        self.description = None
        self._total_rowcount = -1
        self._arraysize = 1  # PEP-0249: defaults to 1
        self._closed = False
        self._use_dict_result = use_dict_result
        self._use_numpy = use_numpy
        # Streaming state for Arrow results
        self._stream_iterator = None
        self._result = None  # Iterator over stream results
        self._rownumber = None  # Current row position in result set
        self.execute_result = None

    @property
    def description(self):
        """
        Read-only attribute describing the result columns of a query.

        Returns:
            tuple: Sequence of 7-item tuples describing each result column:
                   (name, type_code, display_size, internal_size, precision, scale, null_ok)
        """
        return self._description

    @description.setter
    def description(self, value):
        self._description = value

    @property
    def rowcount(self) -> int | None:
        """
        Read-only attribute specifying the number of rows that the last
        .execute*() produced or affected.

        Returns:
            int: Number of rows affected, or None if not determined
        """
        return self._total_rowcount if self._total_rowcount >= 0 else None

    @property
    def rownumber(self) -> int | None:
        """
        Read-only attribute specifying the current 0-based index of the cursor
        in the result set.

        Returns:
            int: Current row index, or None if not determined
        """
        return (
            self._rownumber
            if self._rownumber is not None and self._rownumber >= 0
            else None
        )

    @property
    def arraysize(self) -> int:
        """
        Read/write attribute specifying the number of rows to fetch at a time
        with fetchmany(). It defaults to 1.

        Returns:
            int: Number of rows to fetch
        """
        return self._arraysize

    @arraysize.setter
    def arraysize(self, value) -> None:
        self._arraysize = int(value)

    def callproc(self, procname, parameters=None):
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

    def close(self):
        """
        Close the cursor now (rather than whenever __del__ is called).
        """
        self._closed = True

    def execute(self, operation, parameters=None):
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

        # Update rowcount from execute result
        self._total_rowcount = self.execute_result.rows_affected

        # Reset streaming state for a new result
        self._stream_iterator = None
        self._result = None
        self._rownumber = -1

    def executemany(self, operation, seq_of_parameters):
        """
        Execute a database operation repeatedly for each element in seq_of_parameters.

        Args:
            operation (str): SQL statement to execute
            seq_of_parameters (sequence): Sequence of parameter sequences

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("executemany is not implemented")

    def _ensure_stream_iterator(self):
        if self._stream_iterator is None:
            stream_ptr = int.from_bytes(
                self.execute_result.stream.value, byteorder="little", signed=False
            )
            self._stream_iterator = ArrowStreamIterator(
                stream_ptr,
                ArrowConverterContext(),
                use_dict_result=self._use_dict_result,
                use_numpy=self._use_numpy,
            )

    def fetchone(self):
        """
        Fetch the next row of a query result set.

        Returns:
            tuple: Next row, or None when no more data is available
        """
        # Lazily create iterator on first fetch
        if self._result is None:
            self._ensure_stream_iterator()
            self._result = iter(self._stream_iterator)

        try:
            row = next(self._result)
            if self._rownumber is not None:
                self._rownumber += 1
            return row
        except StopIteration:
            return None

    def fetchmany(self, size=None):
        """
        Fetch the next set of rows of a query result.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            list: List of rows (tuples)
        """
        if size is None:
            size = self.arraysize

        if size < 0:
            raise ProgrammingError(
                f"The number of rows is not zero or positive number: {size}"
            )

        rows = []
        while size > 0:
            row = self.fetchone()
            if row is None:
                break
            rows.append(row)
            size -= 1

        return rows

    def fetchall(self):
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            list: List of all remaining rows (tuples)
        """
        rows = []
        while True:
            row = self.fetchone()
            if row is None:
                break
            rows.append(row)
        return rows

    def nextset(self):
        """
        Skip to the next available set, discarding any remaining rows from current set.

        Returns:
            bool: True if next set is available, False/None otherwise

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("nextset is not implemented")

    def setinputsizes(self, sizes):
        """
        Predefine memory areas for the operation parameters.

        Args:
            sizes (sequence): Sequence of type objects or integers
        """
        # This method is optional and can be implemented as a no-op
        pass

    def setoutputsize(self, size, column=None):
        """
        Set a column buffer size for fetches of large columns.

        Args:
            size (int): Buffer size
            column (int): Column index (optional)
        """
        # This method is optional and can be implemented as a no-op
        pass

    def __iter__(self):
        """
        Iteration over the result set.

        Yields rows from the result set until exhausted.
        """
        while True:
            row = self.fetchone()
            if row is None:
                break
            yield row

    def __enter__(self):
        """
        Enter the runtime context for the cursor.

        Returns:
            Cursor: Self
        """
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """
        Exit the runtime context for the cursor.
        """
        self.close()

    def is_closed(self):
        """
        Check if the cursor is closed.

        Returns:
            bool: True if closed, False otherwise
        """
        return self._closed
