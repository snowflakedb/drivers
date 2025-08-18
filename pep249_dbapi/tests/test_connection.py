"""
Tests for PEP 249 Connection objects.
"""
import pytest

from pep249_dbapi.connection import Connection
from pep249_dbapi.cursor import Cursor
from pep249_dbapi.exceptions import NotSupportedError, InterfaceError


class TestConnectionMethods:
    """Test Connection object methods."""

    def test_close_connection(self, mock_connection):
        """Test closing a connection."""
        conn = mock_connection
        assert not conn._closed
        conn.close()
        assert conn._closed

    def test_commit_not_implemented(self, mock_connection):
        """Test that commit raises NotSupportedError."""
        conn = mock_connection
        with pytest.raises(NotSupportedError) as excinfo:
            conn.commit()
        assert "commit is not implemented" in str(excinfo.value)

    def test_rollback_not_implemented(self, mock_connection):
        """Test that rollback raises NotSupportedError."""
        conn = mock_connection
        with pytest.raises(NotSupportedError) as excinfo:
            conn.rollback()
        assert "rollback is not implemented" in str(excinfo.value)
    
    def test_cursor_creation(self, mock_connection):
        """Test creating a cursor from connection."""
        conn = mock_connection
        cursor = conn.cursor()
        assert isinstance(cursor, Cursor)
        assert cursor.connection is conn
    
    def test_cursor_creation_on_closed_connection(self, mock_connection):
        """Test that creating cursor on closed connection raises error."""
        conn = mock_connection
        conn.close()
        with pytest.raises(InterfaceError) as excinfo:
            conn.cursor()
        assert "Connection is closed" in str(excinfo.value)


class TestConnectionContextManager:
    """Test Connection context manager functionality."""
    
    def test_context_manager_entry(self, mock_connection):
        """Test entering connection context manager."""
        conn = mock_connection
        with conn as c:
            assert c is conn
    
    def test_context_manager_exit_success(self, mock_connection):
        """Test exiting connection context manager successfully."""
        conn = mock_connection
        
        # Mock commit to not raise an exception
        commit_called = False
        def mock_commit():
            nonlocal commit_called
            commit_called = True
        
        conn.commit = mock_commit
        
        with conn:
            pass
        
        assert commit_called
        assert conn._closed
    
    def test_context_manager_exit_with_exception(self, mock_connection):
        """Test exiting connection context manager with exception."""
        conn = mock_connection
        
        # Mock rollback to not raise an exception
        rollback_called = False
        def mock_rollback():
            nonlocal rollback_called
            rollback_called = True
        
        conn.rollback = mock_rollback
        
        try:
            with conn:
                raise ValueError("Test exception")
        except ValueError:
            pass
        
        assert rollback_called
        assert conn._closed
    
    def test_context_manager_handles_not_supported_commit(self, mock_connection):
        """Test context manager handles NotSupportedError from commit."""
        conn = mock_connection

        # Default commit raises NotSupportedError, should be handled gracefully
        with conn:
            pass
        
        assert conn._closed
    
    def test_context_manager_handles_not_supported_rollback(self, mock_connection):
        """Test context manager handles NotSupportedError from rollback."""
        conn = mock_connection
        
        # Default rollback raises NotSupportedError, should be handled gracefully
        try:
            with conn:
                raise ValueError("Test exception")
        except ValueError:
            pass
        
        assert conn._closed


class TestConnectionOptionalMethods:
    """Test optional Connection methods."""
    
    def test_cancel_not_implemented(self, mock_connection):
        """Test that cancel raises NotSupportedError."""
        conn = mock_connection
        with pytest.raises(NotSupportedError) as excinfo:
            conn.cancel()
        assert "cancel is not implemented" in str(excinfo.value)
    
    def test_ping_not_implemented(self, mock_connection):
        """Test that ping raises NotSupportedError."""
        conn = mock_connection
        with pytest.raises(NotSupportedError) as excinfo:
            conn.ping()
        assert "ping is not implemented" in str(excinfo.value)
    
    def test_set_autocommit_not_implemented(self, mock_connection):
        """Test that set_autocommit raises NotSupportedError."""
        conn = mock_connection
        with pytest.raises(NotSupportedError) as excinfo:
            conn.set_autocommit(True)
        assert "set_autocommit is not implemented" in str(excinfo.value)
    
    def test_get_autocommit_not_implemented(self, mock_connection):
        """Test that get_autocommit raises NotSupportedError."""
        conn = mock_connection
        with pytest.raises(NotSupportedError) as excinfo:
            conn.get_autocommit()
        assert "get_autocommit is not implemented" in str(excinfo.value)


class TestConnectionAutocommitProperty:
    """Test Connection autocommit property."""
    
    def test_autocommit_property_get(self, mock_connection):
        """Test getting autocommit property."""
        conn = mock_connection
        assert conn.autocommit is False
        
        conn._autocommit = True
        assert conn.autocommit is True
    
    def test_autocommit_property_set(self, mock_connection):
        """Test setting autocommit property."""
        conn = mock_connection
        
        # Mock set_autocommit to track calls
        set_autocommit_called = False
        set_autocommit_value = None
        
        def mock_set_autocommit(value):
            nonlocal set_autocommit_called, set_autocommit_value
            set_autocommit_called = True
            set_autocommit_value = value
        
        conn.set_autocommit = mock_set_autocommit
        
        conn.autocommit = True
        
        assert conn._autocommit is True
        assert set_autocommit_called
        assert set_autocommit_value is True
    
    def test_autocommit_property_set_handles_not_supported(self, mock_connection):
        """Test setting autocommit property handles NotSupportedError."""
        conn = mock_connection
        
        # Default set_autocommit raises NotSupportedError
        conn.autocommit = True
        
        # Should set internal flag despite NotSupportedError
        assert conn._autocommit is True


@pytest.mark.integration
class TestIntegrationConnection:
    """Integration tests for Connection with real database connections."""
    
    def test_connection_creation(self, connection):
        """Test creating a real connection."""
        assert connection is not None
        assert hasattr(connection, 'cursor')
        
    def test_cursor_creation(self, connection):
        """Test creating a cursor from real connection."""
        cursor = connection.cursor()
        assert cursor is not None
        assert hasattr(cursor, 'execute')

    def test_simple_select(self, cursor):
        """Test simple SELECT query."""
        cursor.execute("SELECT 1 as test_column")
        result = cursor.fetchone()
        # Result format may vary between connectors
        assert result is not None
        
    def test_connection_info_query(self, cursor):
        """Test querying connection information."""
        cursor.execute("SELECT CURRENT_VERSION()")
        result = cursor.fetchone()
        assert result is not None 