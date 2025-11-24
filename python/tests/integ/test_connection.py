"""
Integration tests for PEP 249 Connection objects.
"""
import pytest
from unittest.mock import Mock

from snowflake.ud_connector.exceptions import NotSupportedError


class TestConnectionMethods:
    """Test Connection object methods."""

    def test_close_connection(self, connection):
        """Test closing a connection."""
        assert not connection.is_closed()
        connection.close()
        assert connection.is_closed()

    @pytest.mark.skip_reference
    def test_commit_not_implemented(self, connection):
        """Test that commit raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.commit()
        assert "commit is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
    def test_rollback_not_implemented(self, connection):
        """Test that rollback raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.rollback()
        assert "rollback is not implemented" in str(excinfo.value)

# TODO: Tests for context manager were deleted - we might want to add them again later

class TestConnectionOptionalMethods:
    """Test optional Connection methods."""

    @pytest.mark.skip_reference
    def test_cancel_not_implemented(self, connection):
        """Test that cancel raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.cancel()
        assert "cancel is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
    def test_ping_not_implemented(self, connection):
        """Test that ping raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.ping()
        assert "ping is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
    def test_set_autocommit_not_implemented(self, connection):
        """Test that set_autocommit raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.set_autocommit(True)
        assert "set_autocommit is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
    def test_get_autocommit_not_implemented(self, connection):
        """Test that get_autocommit raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.get_autocommit()
        assert "get_autocommit is not implemented" in str(excinfo.value)


class TestConnectionAutocommitProperty:
    """Test Connection autocommit property."""

    @pytest.mark.skip_reference
    def test_autocommit_property_get(self, connection):
        """Test getting autocommit property."""
        assert connection.autocommit is False
        
        connection._autocommit = True
        assert connection.autocommit is True

    @pytest.mark.skip_reference
    def test_autocommit_property_set(self, connection, monkeypatch):
        """Test setting autocommit property."""
        # Mock set_autocommit to track calls
        mock_set_autocommit = Mock()
        monkeypatch.setattr(connection, 'set_autocommit', mock_set_autocommit)
        
        connection.autocommit = True
        
        assert connection._autocommit is True
        mock_set_autocommit.assert_called_once_with(True)

    @pytest.mark.skip_reference
    def test_autocommit_property_set_handles_not_supported(self, connection):
        """Test setting autocommit property handles NotSupportedError."""
        # Default set_autocommit raises NotSupportedError
        connection.autocommit = True
        
        # Should set internal flag despite NotSupportedError
        assert connection._autocommit is True
