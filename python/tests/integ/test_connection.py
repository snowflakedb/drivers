"""
Integration tests for PEP 249 Connection objects.
"""

from unittest.mock import Mock

import pytest

from snowflake.connector.errors import NotSupportedError


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
    def test_set_autocommit(self, connection):
        """Test that set_autocommit sets the internal flag."""
        assert connection._autocommit is False
        connection.set_autocommit(True)
        assert connection._autocommit is True

    @pytest.mark.skip_reference
    def test_get_autocommit(self, connection):
        """Test that get_autocommit returns the current setting."""
        assert connection.get_autocommit() is False
        connection._autocommit = True
        assert connection.get_autocommit() is True


class TestConnectionAutocommitMethod:
    """Test Connection autocommit method."""

    @pytest.mark.skip_reference
    def test_autocommit_sets_flag_and_calls_set_autocommit(self, connection, monkeypatch):
        """Test that autocommit() sets _autocommit and delegates to set_autocommit."""
        mock_set_autocommit = Mock()
        monkeypatch.setattr(connection, "set_autocommit", mock_set_autocommit)

        connection.autocommit(True)

        assert connection._autocommit is True
        mock_set_autocommit.assert_called_once_with(True)

    @pytest.mark.skip_reference
    def test_autocommit_default_is_false(self, connection):
        """Test that autocommit defaults to False."""
        assert connection._autocommit is False

    @pytest.mark.skip_reference
    def test_autocommit_roundtrip(self, connection):
        """Test setting autocommit via autocommit() and reading via get_autocommit()."""
        connection.autocommit(True)
        assert connection.get_autocommit() is True

        connection.autocommit(False)
        assert connection.get_autocommit() is False
