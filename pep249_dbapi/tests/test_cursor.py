"""
Tests for PEP 249 Cursor objects.
"""

import pytest

from pep249_dbapi.connection import Connection
from pep249_dbapi.cursor import Cursor
from pep249_dbapi.exceptions import NotSupportedError
from . import create_connection


class TestCursorProperties:
    """Test Cursor object properties."""
    
    def test_description_property(self):
        """Test description property getter and setter."""
        conn = create_connection()
        cursor = Cursor(conn)
        
        # Test initial value
        assert cursor.description is None
        
        # Test setting value
        test_description = [
            ("col1", "STRING", None, None, None, None, True),
            ("col2", "INTEGER", None, None, None, None, False)
        ]
        cursor.description = test_description
        assert cursor.description == test_description
    
    def test_rowcount_property(self):
        """Test rowcount property getter and setter."""
        conn = create_connection()
        cursor = Cursor(conn)
        
        # Test initial value
        assert cursor.rowcount == -1
        
        # Test setting value
        cursor.rowcount = 42
        assert cursor.rowcount == 42

# @pytest.mark.skip(reason="Cursor is not implemented")
class TestCursorMethods:
    """Test Cursor object methods."""
    
    def test_close_cursor(self):
        """Test closing a cursor."""
        conn = create_connection()
        cursor = Cursor(conn)
        assert not cursor._closed
        cursor.close()
        assert cursor._closed
    
    def test_callproc_not_implemented(self):
        """Test that callproc raises NotSupportedError."""
        conn = create_connection()
        cursor = Cursor(conn)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.callproc("test_proc", [1, 2, 3])
        assert "callproc is not implemented" in str(excinfo.value)
    
    def test_executemany_not_implemented(self):
        """Test that executemany raises NotSupportedError."""
        conn = create_connection()
        cursor = Cursor(conn)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.executemany("INSERT INTO test VALUES (?)", [(1,), (2,)])
        assert "executemany is not implemented" in str(excinfo.value)
    
    def test_fetchmany_not_implemented(self):
        """Test that fetchmany raises NotSupportedError."""
        conn = create_connection()
        cursor = Cursor(conn)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany()
        assert "fetchmany is not implemented" in str(excinfo.value)
    
    def test_fetchmany_with_size_not_implemented(self):
        """Test that fetchmany with size raises NotSupportedError."""
        conn = create_connection()
        cursor = Cursor(conn)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany(5)
        assert "fetchmany is not implemented" in str(excinfo.value)
    
    def test_nextset_not_implemented(self):
        """Test that nextset raises NotSupportedError."""
        conn = create_connection()
        cursor = Cursor(conn)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.nextset()
        assert "nextset is not implemented" in str(excinfo.value)
    
    def test_setinputsizes_no_op(self):
        """Test that setinputsizes is a no-op."""
        conn = create_connection()
        cursor = Cursor(conn)
        # Should not raise any exception
        cursor.setinputsizes([10, 20, 30])
    
    def test_setoutputsize_no_op(self):
        """Test that setoutputsize is a no-op."""
        conn = create_connection()
        cursor = Cursor(conn)
        # Should not raise any exception
        cursor.setoutputsize(100)
        cursor.setoutputsize(100, 1)

class TestCursorIterator:
    """Test Cursor iterator protocol."""
    
    def test_cursor_is_iterator(self):
        """Test that cursor returns itself as iterator."""
        conn = create_connection()
        cursor = Cursor(conn)
        assert iter(cursor) is cursor
    
    def test_cursor_next_calls_fetchone(self):
        """Test that __next__ calls fetchone."""
        conn = create_connection()
        cursor = Cursor(conn)
        
        # Mock fetchone to return a test row, then None
        call_count = 0
        def mock_fetchone():
            nonlocal call_count
            call_count += 1
            if call_count == 1:
                return ("test", "row")
            return None
        
        cursor.fetchone = mock_fetchone
        
        # First call should return the row
        row = next(cursor)
        assert row == ("test", "row")
        
        # Second call should raise StopIteration
        with pytest.raises(StopIteration):
            next(cursor)
    
    def test_cursor_iteration_with_multiple_rows(self):
        """Test cursor iteration with multiple rows."""
        conn = create_connection()
        cursor = Cursor(conn)
        
        # Mock fetchone to return test rows
        test_rows = [("row1",), ("row2",), ("row3",)]
        row_index = 0
        
        def mock_fetchone():
            nonlocal row_index
            if row_index < len(test_rows):
                row = test_rows[row_index]
                row_index += 1
                return row
            return None
        
        cursor.fetchone = mock_fetchone
        
        # Collect all rows
        rows = list(cursor)
        assert rows == test_rows

class TestCursorContextManager:
    """Test Cursor context manager functionality."""
    
    def test_context_manager_entry(self):
        """Test entering cursor context manager."""
        conn = create_connection()
        cursor = Cursor(conn)
        with cursor as c:
            assert c is cursor
    
    def test_context_manager_exit(self):
        """Test exiting cursor context manager."""
        conn = create_connection()
        cursor = Cursor(conn)
        
        with cursor:
            pass
        
        assert cursor._closed
    
    def test_context_manager_exit_with_exception(self):
        """Test exiting cursor context manager with exception."""
        conn = create_connection()
        cursor = Cursor(conn)
        
        try:
            with cursor:
                raise ValueError("Test exception")
        except ValueError:
            pass
        
        assert cursor._closed

class TestCursorPython2Compatibility:
    """Test Python 2 compatibility features."""
    
    def test_next_method_exists(self):
        """Test that 'next' method exists for Python 2 compatibility."""
        conn = create_connection()
        cursor = Cursor(conn)
        
        # Should have both __next__ and next
        assert hasattr(cursor, '__next__')
        assert hasattr(cursor, 'next')
        assert callable(cursor.next)
        
        # Test that next() calls __next__() by mocking fetchone
        call_count = 0
        def mock_fetchone():
            nonlocal call_count
            call_count += 1
            if call_count == 1:
                return ("test", "row")
            return None
        
        cursor.fetchone = mock_fetchone
        
        # Both next() and __next__() should work the same way
        row1 = cursor.next()
        assert row1 == ("test", "row")
        
        # Reset for __next__ test
        call_count = 0
        row2 = cursor.__next__()
        assert row2 == ("test", "row") 

class TestCursorSimpleSelect:
    """Test Cursor simple select."""

    def test_simple_select(self):
        """Test simple select."""
        conn = create_connection()
        cursor = Cursor(conn)
        cursor.execute("SELECT 1")
        assert cursor.fetchone() == (1,)


class TestCursorLargeResult:
    """Test Cursor large result."""

    data_sizes = [100000, 1000000]

    @pytest.mark.parametrize("data_size", data_sizes)
    def test_large_result(self, data_size):
        """Test large result."""
        conn = create_connection()
        cursor = Cursor(conn)
        cursor.execute(f"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => {data_size})) v ORDER BY id")
        rows = cursor.fetchall()
        assert len(rows) == data_size
        for (i, row) in enumerate(rows):
            assert row == (i,)

