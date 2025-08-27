"""
Integration tests for PEP 249 Connection objects.
"""
import pytest
from unittest.mock import Mock

from pep249_dbapi.connection import Connection
from pep249_dbapi.cursor import Cursor
from pep249_dbapi.exceptions import NotSupportedError, InterfaceError


class TestConnectionMethods:
    """Test Connection object methods."""

    def test_close_connection(self, connection):
        """Test closing a connection."""
        assert not connection._closed
        connection.close()
        assert connection._closed

    def test_commit_not_implemented(self, connection):
        """Test that commit raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.commit()
        assert "commit is not implemented" in str(excinfo.value)

    def test_rollback_not_implemented(self, connection):
        """Test that rollback raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.rollback()
        assert "rollback is not implemented" in str(excinfo.value)
    
    def test_cursor_creation(self, connection):
        """Test creating a cursor from connection."""
        cursor = connection.cursor()
        assert isinstance(cursor, Cursor)
        assert cursor.connection is connection
    
    def test_cursor_creation_on_closed_connection(self, connection):
        """Test that creating cursor on closed connection raises error."""
        connection.close()
        with pytest.raises(InterfaceError) as excinfo:
            connection.cursor()
        assert "Connection is closed" in str(excinfo.value)


class TestConnectionContextManager:
    """Test Connection context manager functionality."""
    
    def test_context_manager_entry(self, connection):
        """Test entering connection context manager."""
        with connection as c:
            assert c is connection
    
    def test_context_manager_exit_success(self, connection, monkeypatch):
        """Test exiting connection context manager successfully."""
        # Mock commit to not raise an exception
        mock_commit = Mock()
        monkeypatch.setattr(connection, 'commit', mock_commit)
        
        with connection:
            pass
        
        mock_commit.assert_called_once()
        assert connection._closed
    
    def test_context_manager_exit_with_exception(self, connection, monkeypatch):
        """Test exiting connection context manager with exception."""
        # Mock rollback to not raise an exception
        mock_rollback = Mock()
        monkeypatch.setattr(connection, 'rollback', mock_rollback)
        
        try:
            with connection:
                raise ValueError("Test exception")
        except ValueError:
            pass
        
        mock_rollback.assert_called_once()
        assert connection._closed
    
    def test_context_manager_handles_not_supported_commit(self, connection):
        """Test context manager handles NotSupportedError from commit."""
        # Default commit raises NotSupportedError, should be handled gracefully
        with connection:
            pass
        
        assert connection._closed
    
    def test_context_manager_handles_not_supported_rollback(self, connection):
        """Test context manager handles NotSupportedError from rollback."""
        # Default rollback raises NotSupportedError, should be handled gracefully
        try:
            with connection:
                raise ValueError("Test exception")
        except ValueError:
            pass
        
        assert connection._closed


class TestConnectionOptionalMethods:
    """Test optional Connection methods."""
    
    def test_cancel_not_implemented(self, connection):
        """Test that cancel raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.cancel()
        assert "cancel is not implemented" in str(excinfo.value)
    
    def test_ping_not_implemented(self, connection):
        """Test that ping raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.ping()
        assert "ping is not implemented" in str(excinfo.value)
    
    def test_set_autocommit_not_implemented(self, connection):
        """Test that set_autocommit raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.set_autocommit(True)
        assert "set_autocommit is not implemented" in str(excinfo.value)
    
    def test_get_autocommit_not_implemented(self, connection):
        """Test that get_autocommit raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.get_autocommit()
        assert "get_autocommit is not implemented" in str(excinfo.value)


class TestConnectionAutocommitProperty:
    """Test Connection autocommit property."""
    
    def test_autocommit_property_get(self, connection):
        """Test getting autocommit property."""
        assert connection.autocommit is False
        
        connection._autocommit = True
        assert connection.autocommit is True
    
    def test_autocommit_property_set(self, connection, monkeypatch):
        """Test setting autocommit property."""
        # Mock set_autocommit to track calls
        mock_set_autocommit = Mock()
        monkeypatch.setattr(connection, 'set_autocommit', mock_set_autocommit)
        
        connection.autocommit = True
        
        assert connection._autocommit is True
        mock_set_autocommit.assert_called_once_with(True)
    
    def test_autocommit_property_set_handles_not_supported(self, connection):
        """Test setting autocommit property handles NotSupportedError."""
        # Default set_autocommit raises NotSupportedError
        connection.autocommit = True
        
        # Should set internal flag despite NotSupportedError
        assert connection._autocommit is True
