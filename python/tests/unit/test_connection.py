"""
Unit tests for Connection.
"""

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
)
from snowflake.connector.errors import ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


@pytest.fixture
def mock_db_api():
    """Create a mock DatabaseDriverClient with minimal stubs for Connection.__init__."""
    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    return db_api


@pytest.fixture
def connection(mock_db_api):
    """Create a Connection with a mocked db_api, bypassing the real sf_core."""
    from snowflake.connector.connection import Connection

    with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
        conn = Connection(user="test_user", account="test_account")
    return conn


class TestGetConnectionInfo:
    """Unit tests for Connection._get_connection_info."""

    def test_queries_sf_core_on_each_call(self, connection, mock_db_api):
        """Each call to _get_connection_info should invoke db_api.connection_get_info."""
        connection._get_connection_info()
        connection._get_connection_info()
        connection._get_connection_info()

        assert mock_db_api.connection_get_info.call_count == 3

    def test_returns_fresh_response_each_time(self, connection, mock_db_api):
        """Successive calls should return whatever sf_core returns, not a cached value."""
        first_response = MagicMock(host="host-a", session_token="token-1")
        second_response = MagicMock(host="host-b", session_token="token-2")
        mock_db_api.connection_get_info.side_effect = [first_response, second_response]

        result1 = connection._get_connection_info()
        result2 = connection._get_connection_info()

        assert result1.host == "host-a"
        assert result1.session_token == "token-1"
        assert result2.host == "host-b"
        assert result2.session_token == "token-2"

    def test_passes_correct_conn_handle(self, connection, mock_db_api):
        """The request should carry the connection's conn_handle."""
        mock_db_api.connection_get_info.return_value = MagicMock()

        connection._get_connection_info()

        args, _ = mock_db_api.connection_get_info.call_args
        assert args[0].conn_handle == connection.conn_handle


class TestSetAutocommitValidation:
    """Unit tests for set_autocommit input validation."""

    def test_set_autocommit_rejects_non_bool(self, connection):
        """set_autocommit should raise ProgrammingError for non-bool input."""
        with pytest.raises(ProgrammingError, match="Invalid parameter"):
            connection.set_autocommit("yes")

        with pytest.raises(ProgrammingError, match="Invalid parameter"):
            connection.set_autocommit(1)

    def test_init_autocommit_kwarg_rejects_non_bool(self, mock_db_api):
        """Connection(autocommit=1) should raise ProgrammingError."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            with pytest.raises(ProgrammingError, match="Invalid autocommit parameter"):
                Connection(user="test_user", account="test_account", autocommit=1)


class TestAutocommitKwargUnit:
    """Unit tests for the autocommit keyword argument at connection time."""

    def test_autocommit_true_injects_session_parameter(self, mock_db_api):
        """Connection(autocommit=True) should pass AUTOCOMMIT=true as a session parameter."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            conn = Connection(user="test_user", account="test_account", autocommit=True)

        assert conn.get_autocommit() is True
        call_args = mock_db_api.connection_set_session_parameters.call_args
        params = call_args[0][0].parameters
        assert params["AUTOCOMMIT"] == "true"

    def test_autocommit_false_injects_session_parameter(self, mock_db_api):
        """Connection(autocommit=False) should pass AUTOCOMMIT=false as a session parameter."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            conn = Connection(user="test_user", account="test_account", autocommit=False)

        assert conn.get_autocommit() is False
        call_args = mock_db_api.connection_set_session_parameters.call_args
        params = call_args[0][0].parameters
        assert params["AUTOCOMMIT"] == "false"

    def test_no_autocommit_kwarg_does_not_set_session_parameter(self, mock_db_api):
        """Connection without autocommit kwarg should not inject AUTOCOMMIT session parameter."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            conn = Connection(user="test_user", account="test_account")

        assert conn.get_autocommit() is False
        # No session parameters should be set at all
        mock_db_api.connection_set_session_parameters.assert_not_called()


class TestContextManagerUnit:
    """Unit tests for __exit__ behavior."""

    def test_exit_skips_commit_when_autocommit_on(self, connection, mock_db_api):
        """When autocommit is on, __exit__ should not execute COMMIT or ROLLBACK."""
        connection._autocommit = True
        mock_db_api.reset_mock()

        connection.__exit__(None, None, None)

        # No statement_execute calls for COMMIT/ROLLBACK
        for call in mock_db_api.method_calls:
            if "statement_execute" in str(call):
                assert "COMMIT" not in str(call)
                assert "ROLLBACK" not in str(call)

    def test_exit_always_closes(self, connection):
        """close() should be called even if commit raises an exception."""
        connection._autocommit = False

        def failing_commit():
            raise RuntimeError("commit failed")

        connection.commit = failing_commit

        with pytest.raises(RuntimeError, match="commit failed"):
            connection.__exit__(None, None, None)

        assert connection._closed is True
