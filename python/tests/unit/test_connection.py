"""
Unit tests for Connection._get_connection_info.
"""

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
)
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
