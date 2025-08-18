"""
Tests for PEP 249 Cursor objects.
"""

import pytest

from pep249_dbapi.connection import Connection
from pep249_dbapi.cursor import Cursor
from pep249_dbapi.exceptions import NotSupportedError


class TestCursorProperties:
    """Test Cursor object properties."""
    
    def test_description_property(self, mock_connection):
        """Test description property getter and setter."""
        cursor = Cursor(mock_connection)
        
        # Test initial value
        assert cursor.description is None
        
        # Test setting value
        test_description = [
            ("col1", "STRING", None, None, None, None, True),
            ("col2", "INTEGER", None, None, None, None, False)
        ]
        cursor.description = test_description
        assert cursor.description == test_description
    
    def test_rowcount_property(self, mock_connection):
        """Test rowcount property getter and setter."""
        cursor = Cursor(mock_connection)
        
        # Test initial value
        assert cursor.rowcount == -1
        
        # Test setting value
        cursor.rowcount = 42
        assert cursor.rowcount == 42

# @pytest.mark.skip(reason="Cursor is not implemented")
class TestCursorMethods:
    """Test Cursor object methods."""
    
    def test_close_cursor(self, mock_connection):
        """Test closing a cursor."""
        cursor = Cursor(mock_connection)
        assert not cursor._closed
        cursor.close()
        assert cursor._closed
    
    def test_callproc_not_implemented(self, mock_connection):
        """Test that callproc raises NotSupportedError."""
        cursor = Cursor(mock_connection)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.callproc("test_proc", [1, 2, 3])
        assert "callproc is not implemented" in str(excinfo.value)
    
    def test_executemany_not_implemented(self, mock_connection):
        """Test that executemany raises NotSupportedError."""
        cursor = Cursor(mock_connection)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.executemany("INSERT INTO test VALUES (?)", [(1,), (2,)])
        assert "executemany is not implemented" in str(excinfo.value)
    
    def test_fetchmany_not_implemented(self, mock_connection):
        """Test that fetchmany raises NotSupportedError."""
        cursor = Cursor(mock_connection)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany()
        assert "fetchmany is not implemented" in str(excinfo.value)
    
    def test_fetchmany_with_size_not_implemented(self, mock_connection):
        """Test that fetchmany with size raises NotSupportedError."""
        cursor = Cursor(mock_connection)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany(5)
        assert "fetchmany is not implemented" in str(excinfo.value)
    
    def test_nextset_not_implemented(self, mock_connection):
        """Test that nextset raises NotSupportedError."""
        cursor = Cursor(mock_connection)
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.nextset()
        assert "nextset is not implemented" in str(excinfo.value)
    
    def test_setinputsizes_no_op(self, mock_connection):
        """Test that setinputsizes is a no-op."""
        cursor = Cursor(mock_connection)
        # Should not raise any exception
        cursor.setinputsizes([10, 20, 30])
    
    def test_setoutputsize_no_op(self, mock_connection):
        """Test that setoutputsize is a no-op."""
        cursor = Cursor(mock_connection)
        # Should not raise any exception
        cursor.setoutputsize(100)
        cursor.setoutputsize(100, 1)

class TestCursorIterator:
    """Test Cursor iterator protocol."""
    
    def test_cursor_is_iterator(self, mock_connection):
        """Test that cursor returns itself as iterator."""
        cursor = Cursor(mock_connection)
        assert iter(cursor) is cursor
    
    def test_cursor_next_calls_fetchone(self, mock_connection):
        """Test that __next__ calls fetchone."""
        cursor = Cursor(mock_connection)
        
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
    
    def test_cursor_iteration_with_multiple_rows(self, mock_connection):
        """Test cursor iteration with multiple rows."""
        cursor = Cursor(mock_connection)
        
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
    
    def test_context_manager_entry(self, mock_connection):
        """Test entering cursor context manager."""
        cursor = Cursor(mock_connection)
        with cursor as c:
            assert c is cursor
    
    def test_context_manager_exit(self, mock_connection):
        """Test exiting cursor context manager."""
        cursor = Cursor(mock_connection)
        
        with cursor:
            pass
        
        assert cursor._closed
    
    def test_context_manager_exit_with_exception(self, mock_connection):
        """Test exiting cursor context manager with exception."""
        cursor = Cursor(mock_connection)
        
        try:
            with cursor:
                raise ValueError("Test exception")
        except ValueError:
            pass
        
        assert cursor._closed

class TestCursorPython2Compatibility:
    """Test Python 2 compatibility features."""
    
    def test_next_method_exists(self, mock_connection):
        """Test that 'next' method exists for Python 2 compatibility."""
        cursor = Cursor(mock_connection)
        
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

@pytest.mark.integration
class TestIntegrationCursor:
    """Integration tests for Cursor with real database queries."""

    def test_simple_select(self, cursor):
        """Test simple select."""
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        # Result format may vary between connectors, just check it's not None
        assert result is not None

    def test_current_version_select(self, cursor):
        """Test querying current version."""
        cursor.execute("SELECT CURRENT_VERSION()")
        result = cursor.fetchone()
        assert result is not None

    @pytest.mark.slow
    @pytest.mark.parametrize("data_size", [1000, 10000])
    def test_large_result(self, cursor, data_size):
        """Test large result."""
        cursor.execute(f"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => {data_size})) v ORDER BY id")
        rows = cursor.fetchall()
        assert len(rows) == data_size
        # Check first few and last few rows instead of all to be more efficient
        for i in range(min(10, data_size)):
            assert rows[i] == (i,)
        for i in range(max(0, data_size - 10), data_size):
            assert rows[i] == (i,)

