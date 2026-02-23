"""
Integration tests for PEP 249 Connection objects.
"""

from unittest.mock import Mock

import pytest

from snowflake.connector.errors import NotSupportedError


class TestConnectionInfo:
    """Integration tests for Connection._connection_info property."""

    @pytest.mark.skip_reference
    def test_connection_info_is_set_after_connect(self, connection):
        """Test that _connection_info is populated after connection is established."""
        # Given an established connection
        # When accessing _connection_info
        info = connection._connection_info

        # Then it should not be None
        assert info is not None

    @pytest.mark.skip_reference
    def test_connection_info_has_host(self, connection):
        """Test that _connection_info contains a host value."""
        # Given an established connection
        info = connection._connection_info

        # When checking the host field
        # Then host should be set and non-empty
        assert info.HasField("host")
        assert isinstance(info.host, str)
        assert len(info.host) > 0

    @pytest.mark.skip_reference
    def test_connection_info_has_port(self, connection):
        """Test that _connection_info contains a port value."""
        # Given an established connection
        info = connection._connection_info

        # When checking the port field
        # Then port should be set and positive
        assert info.HasField("port")
        assert isinstance(info.port, int)
        assert info.port > 0

    @pytest.mark.skip_reference
    def test_connection_info_has_session_token(self, connection):
        """Test that _connection_info contains a session token after login."""
        # Given an established connection
        info = connection._connection_info

        # When checking the session_token field
        # Then session_token should be set and non-empty
        assert info.HasField("session_token")
        assert isinstance(info.session_token, str)
        assert len(info.session_token) > 0

    @pytest.mark.skip_reference
    def test_connection_info_has_session_id(self, connection):
        """Test that _connection_info contains a session ID after login."""
        # Given an established connection
        info = connection._connection_info

        # When checking the session_id field
        # Then session_id should be set
        assert info.HasField("session_id")
        assert isinstance(info.session_id, int)


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
        monkeypatch.setattr(connection, "set_autocommit", mock_set_autocommit)

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
