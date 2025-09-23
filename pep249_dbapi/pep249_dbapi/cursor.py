"""
PEP 249 Database API 2.0 Cursor Objects

This module defines the Cursor class as specified in PEP 249.
"""
from .exceptions import NotSupportedError
import pyarrow
from .protobuf_gen.database_driver_v1_pb2 import *

class Cursor:
    """
    Cursor objects represent a database cursor, which is used to manage the context
    of a fetch operation.
    """
    
    # Class attribute for arraysize
    arraysize = 1
    
    def __init__(self, connection):
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
    def rowcount(self):
        """
        Read-only attribute specifying the number of rows that the last
        .execute*() produced or affected.
        
        Returns:
            int: Number of rows affected, or -1 if not determined
        """
        return self._rowcount
    
    @rowcount.setter  
    def rowcount(self, value):
        self._rowcount = value
    
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
        stmt_handle = self.connection.db_api.statement_new(StatementNewRequest(conn_handle=self.connection.conn_handle)).stmt_handle
        self.connection.db_api.statement_set_sql_query(StatementSetSqlQueryRequest(stmt_handle=stmt_handle, query=operation))
        self.execute_result = self.connection.db_api.statement_execute_query(StatementExecuteQueryRequest(stmt_handle=stmt_handle)).result
        # Reset streaming state for a new result
        self._reader = None
        self._current_batch = None
        self._current_row_in_batch = 0

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

    def _batch_reader(self):
        stream_ptr = int.from_bytes(self.execute_result.stream.value, byteorder="little", signed=False)
        reader = pyarrow.RecordBatchReader._import_from_c(stream_ptr)
        return reader

    def _ensure_reader(self):
        if self._reader is None:
            self._reader = self._batch_reader()

    def fetchone(self):
        """
        Fetch the next row of a query result set.
        
        Returns:
            sequence: Next row, or None when no more data is available
            
        Raises:
            NotSupportedError: If not implemented
        """
        # Initialize reader on first use
        self._ensure_reader()
        # If no current batch or exhausted, read next batch
        while True:
            if self._current_batch is None or self._current_row_in_batch >= self._current_batch.num_rows:
                try:
                    self._current_batch = self._reader.read_next_batch()
                    self._current_row_in_batch = 0
                except StopIteration:
                    return None
            # Produce row from current batch
            row_index = self._current_row_in_batch
            self._current_row_in_batch += 1
            # Handle empty schema
            if self._current_batch.num_columns == 0:
                return tuple()
            values = []
            for col_idx in range(self._current_batch.num_columns):
                values.append(self._current_batch.columns[col_idx][row_index].as_py())
            return tuple(values)

    def fetchmany(self, size=None):
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
    
    def fetchall(self):
        """
        Fetch all (remaining) rows of a query result.
        
        Returns:
            sequence: List of all remaining rows
            
        Raises:
            NotSupportedError: If not implemented
        """
        # Consume remaining rows using current streaming state
        self._ensure_reader()
        rows = []
        # Drain current batch first (if any)
        if self._current_batch is not None:
            while self._current_row_in_batch < self._current_batch.num_rows:
                row_index = self._current_row_in_batch
                self._current_row_in_batch += 1
                values = []
                for column_index in range(self._current_batch.num_columns):
                    values.append(self._current_batch.columns[column_index][row_index].as_py())
                rows.append(tuple(values))
        # Read following batches
        while True:
            try:
                batch = self._reader.read_next_batch()
            except StopIteration:
                break
            for row_index in range(batch.num_rows):
                values = []
                for column_index in range(batch.num_columns):
                    values.append(batch.columns[column_index][row_index].as_py())
                rows.append(tuple(values))
        # Mark stream as exhausted
        self._current_batch = None
        self._current_row_in_batch = 0
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
        Return the cursor itself as an iterator.
        
        Returns:
            Cursor: Self
        """
        return self
    
    def __next__(self):
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
    def next(self):
        """Python 2 compatibility method."""
        return self.__next__()
    
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
