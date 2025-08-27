"""
Integration tests for PEP 249 Cursor objects.
"""

import pytest
from unittest.mock import Mock

from pep249_dbapi.connection import Connection
from pep249_dbapi.cursor import Cursor
from pep249_dbapi.exceptions import NotSupportedError


class TestCursorProperties:
    """Test Cursor object properties."""
    
    def test_description_property(self, cursor):
        """Test description property getter and setter."""
        # Test initial value
        assert cursor.description is None
        
        # Test setting value
        test_description = [
            ("col1", "STRING", None, None, None, None, True),
            ("col2", "INTEGER", None, None, None, None, False)
        ]
        cursor.description = test_description
        assert cursor.description == test_description
    
    def test_rowcount_property(self, cursor):
        """Test rowcount property getter and setter."""
        # Test initial value
        assert cursor.rowcount == -1
        
        # Test setting value
        cursor.rowcount = 42
        assert cursor.rowcount == 42


class TestCursorMethods:
    """Test Cursor object methods."""
    
    def test_close_cursor(self, cursor):
        """Test closing a cursor."""
        assert not cursor._closed
        cursor.close()
        assert cursor._closed
    
    def test_callproc_not_implemented(self, cursor):
        """Test that callproc raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.callproc("test_proc", [1, 2, 3])
        assert "callproc is not implemented" in str(excinfo.value)
    
    def test_executemany_not_implemented(self, cursor):
        """Test that executemany raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.executemany("INSERT INTO test VALUES (?)", [(1,), (2,)])
        assert "executemany is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
    def test_fetchmany_not_implemented(self, cursor):
        """Test that fetchmany raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany()
        assert "fetchmany is not implemented" in str(excinfo.value)
    
    def test_fetchmany_with_size_not_implemented(self, cursor):
        """Test that fetchmany with size raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany(5)
        assert "fetchmany is not implemented" in str(excinfo.value)
    
    def test_nextset_not_implemented(self, cursor):
        """Test that nextset raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.nextset()
        assert "nextset is not implemented" in str(excinfo.value)
    
    def test_setinputsizes_no_op(self, cursor):
        """Test that setinputsizes is a no-op."""
        # Should not raise any exception
        cursor.setinputsizes([10, 20, 30])
    
    def test_setoutputsize_no_op(self, cursor):
        """Test that setoutputsize is a no-op."""
        # Should not raise any exception
        cursor.setoutputsize(100)
        cursor.setoutputsize(100, 1)


class TestCursorIterator:
    """Test Cursor iterator protocol."""
    
    def test_cursor_is_iterator(self, cursor):
        """Test that cursor returns itself as iterator."""
        assert iter(cursor) is cursor
    
    def test_cursor_next_calls_fetchone(self, cursor, monkeypatch):
        """Test that __next__ calls fetchone."""
        # Mock fetchone to return a test row, then None
        mock_fetchone = Mock(side_effect=[("test", "row"), None])
        monkeypatch.setattr(cursor, 'fetchone', mock_fetchone)
        
        # First call should return the row
        row = next(cursor)
        assert row == ("test", "row")
        
        # Second call should raise StopIteration
        with pytest.raises(StopIteration):
            next(cursor)

        # Verify fetchone was called twice
        assert mock_fetchone.call_count == 2

    def test_cursor_iteration_with_multiple_rows(self, cursor, monkeypatch):
        """Test cursor iteration with multiple rows."""
        # Mock fetchone to return test rows
        test_rows = [("row1",), ("row2",), ("row3",)]
        # Add None at the end because PEP 249 cursor iteration calls fetchone()
        # until it returns None to signal end of results
        mock_fetchone = Mock(side_effect=test_rows + [None])
        monkeypatch.setattr(cursor, 'fetchone', mock_fetchone)

        # Collect all rows
        rows = list(cursor)
        assert rows == test_rows

        # Verify fetchone was called for each row plus one final None call
        assert mock_fetchone.call_count == len(test_rows) + 1


class TestCursorContextManager:
    """Test Cursor context manager functionality."""

    def test_context_manager_entry(self, cursor):
        """Test entering cursor context manager."""
        with cursor as c:
            assert c is cursor

    def test_context_manager_exit(self, cursor):
        """Test exiting cursor context manager."""
        with cursor:
            pass

        assert cursor._closed

    def test_context_manager_exit_with_exception(self, cursor):
        """Test exiting cursor context manager with exception."""
        try:
            with cursor:
                raise ValueError("Test exception")
        except ValueError:
            pass

        assert cursor._closed


class TestCursorPython2Compatibility:
    """Test Python 2 compatibility features."""

    def test_next_method_exists(self, cursor, monkeypatch):
        """Test that 'next' method exists for Python 2 compatibility."""
        # Should have both __next__ and next
        assert hasattr(cursor, '__next__')
        assert hasattr(cursor, 'next')
        assert callable(cursor.next)

        # Test that next() calls __next__() by mocking fetchone
        mock_fetchone = Mock(side_effect=[("test", "row"), ("test", "row")])
        monkeypatch.setattr(cursor, 'fetchone', mock_fetchone)

        # Both next() and __next__() should work the same way
        row1 = cursor.next()
        assert row1 == ("test", "row")

        row2 = cursor.__next__()
        assert row2 == ("test", "row")

        # Verify fetchone was called twice
        assert mock_fetchone.call_count == 2


class TestCursorDatabaseQueries:
    """Integration tests for Cursor with real database queries."""

    def test_simple_select(self, cursor):
        """Test simple select."""
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        # Result format may vary between connectors, just check it's not None
        assert result is not None

    @pytest.mark.parametrize("data_size", [1000, 10000])
    def test_large_result(self, cursor, data_size):
        """Test large result."""
        cursor.execute(f"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => {data_size})) v ORDER BY id")
        rows = cursor.fetchall()
        assert len(rows) == data_size

        for (i, row) in enumerate(rows):
            assert row == (i,)