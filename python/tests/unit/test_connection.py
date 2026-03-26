"""
Unit tests for Connection.
"""

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionGetInfoResponse,
    ConnectionHandle,
    DatabaseHandle,
)
from snowflake.connector.constants import QueryStatus
from snowflake.connector.errors import InterfaceError, ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


@pytest.fixture
def mock_db_api():
    """Create a mock DatabaseDriverClient with minimal stubs for Connection.__init__."""
    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    db_api.connection_get_parameter.return_value = MagicMock(value="")
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
        with pytest.raises(ProgrammingError, match="Invalid autocommit parameter"):
            connection.set_autocommit("yes")

        with pytest.raises(ProgrammingError, match="Invalid autocommit parameter"):
            connection.set_autocommit(1)

    def test_init_autocommit_kwarg_rejects_non_bool(self, mock_db_api):
        """Connection(autocommit=1) should raise ProgrammingError."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            with pytest.raises(ProgrammingError, match="Invalid autocommit parameter"):
                Connection(user="test_user", account="test_account", autocommit=1)


class TestSetAutocommit:
    """Unit tests for set_autocommit behavior."""

    def test_set_autocommit_executes_alter_session(self, connection):
        """set_autocommit should execute ALTER SESSION via a cursor."""
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.set_autocommit(True)

        mock_cursor.execute.assert_called_once_with("ALTER SESSION SET autocommit=true")

    def test_set_autocommit_false_executes_alter_session(self, connection):
        """set_autocommit(False) should execute ALTER SESSION with 'false'."""
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.set_autocommit(False)

        mock_cursor.execute.assert_called_once_with("ALTER SESSION SET autocommit=false")

    def test_set_autocommit_closes_cursor_on_error(self, connection):
        """The cursor should be closed even if ALTER SESSION raises."""
        from snowflake.connector.errors import Error

        mock_cursor = MagicMock()
        mock_cursor.execute.side_effect = Error("Autocommit not supported")
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.set_autocommit(True)

        mock_cursor.close.assert_called_once()

    def test_set_autocommit_closes_cursor(self, connection):
        """set_autocommit should always close the cursor, even on success."""
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.set_autocommit(True)

        mock_cursor.close.assert_called_once()


class TestGetAutocommit:
    """Unit tests for get_autocommit behavior."""

    def test_get_autocommit_false_when_param_empty(self, connection):
        """get_autocommit should return False when session parameter is empty/unset."""
        assert connection.get_autocommit() is False

    def test_get_autocommit_reads_from_sf_core(self, connection):
        """get_autocommit should read from sf_core via _get_session_parameter."""
        assert connection.get_autocommit() is False
        connection.db_api.connection_get_parameter.return_value = MagicMock(value="true")
        assert connection.get_autocommit() is True


class TestAutocommitKwargUnit:
    """Unit tests for the autocommit keyword argument at connection time."""

    def test_autocommit_true_injects_session_parameter(self, mock_db_api):
        """Connection(autocommit=True) should pass AUTOCOMMIT=true as a session parameter."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="test_user", account="test_account", autocommit=True)

        call_args = mock_db_api.connection_set_session_parameters.call_args
        params = call_args[0][0].parameters
        assert params["AUTOCOMMIT"] == "true"

    def test_autocommit_false_injects_session_parameter(self, mock_db_api):
        """Connection(autocommit=False) should pass AUTOCOMMIT=false as a session parameter."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="test_user", account="test_account", autocommit=False)

        call_args = mock_db_api.connection_set_session_parameters.call_args
        params = call_args[0][0].parameters
        assert params["AUTOCOMMIT"] == "false"

    def test_no_autocommit_kwarg_does_not_set_autocommit(self, mock_db_api):
        """Connection without autocommit kwarg should not inject AUTOCOMMIT, preserving server default."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="test_user", account="test_account")

        call_args = mock_db_api.connection_set_session_parameters.call_args
        if call_args is not None:
            params = call_args[0][0].parameters
            assert "AUTOCOMMIT" not in params
        # If connection_set_session_parameters was not called at all, that's also correct


class TestContextManagerUnit:
    """Unit tests for __exit__ behavior."""

    def test_exit_skips_commit_when_autocommit_on(self, connection):
        """When autocommit is on, __exit__ should not execute COMMIT or ROLLBACK."""
        connection.db_api.connection_get_parameter.return_value = MagicMock(value="true")
        connection.commit = MagicMock()
        connection.rollback = MagicMock()

        connection.__exit__(None, None, None)

        connection.commit.assert_not_called()
        connection.rollback.assert_not_called()

    def test_exit_always_closes(self, connection):
        """close() should be called even if commit raises an exception."""

        def failing_commit():
            raise RuntimeError("commit failed")

        connection.commit = failing_commit

        with pytest.raises(RuntimeError, match="commit failed"):
            connection.__exit__(None, None, None)

        assert connection._closed is True

    def test_exit_rollback_failure_does_not_mask_original_exception(self, connection):
        """If rollback fails during exception handling, the original exception should propagate."""

        def failing_rollback():
            raise RuntimeError("rollback failed")

        connection.rollback = failing_rollback

        with pytest.raises(ValueError, match="original error"):
            with connection:
                raise ValueError("original error")


class TestConnectionInfoProperties:
    """Unit tests for Connection properties that read from _get_connection_info."""

    @pytest.fixture
    def conn_with_info(self, connection, mock_db_api):
        """Set up a connection with a controllable ConnectionGetInfoResponse."""
        mock_db_api.connection_get_info.return_value = ConnectionGetInfoResponse(
            host="test.snowflakecomputing.com",
            port=443,
            account="test_acct",
            user="test_usr",
            role="SYSADMIN",
            database="MY_DB",
            schema="PUBLIC",
            warehouse="COMPUTE_WH",
            session_id=12345678,
        )
        return connection

    def test_host_returns_value(self, conn_with_info):
        assert conn_with_info.host == "test.snowflakecomputing.com"

    def test_port_returns_value(self, conn_with_info):
        assert conn_with_info.port == 443

    def test_account_returns_value(self, conn_with_info):
        assert conn_with_info.account == "test_acct"

    def test_user_returns_value(self, conn_with_info):
        assert conn_with_info.user == "test_usr"

    def test_role_returns_value(self, conn_with_info):
        assert conn_with_info.role == "SYSADMIN"

    def test_database_returns_value(self, conn_with_info):
        assert conn_with_info.database == "MY_DB"

    def test_schema_returns_value(self, conn_with_info):
        assert conn_with_info.schema == "PUBLIC"

    def test_warehouse_returns_value(self, conn_with_info):
        assert conn_with_info.warehouse == "COMPUTE_WH"

    def test_session_id_returns_value(self, conn_with_info):
        assert conn_with_info.session_id == 12345678


class TestConnectionInfoPropertiesUnset:
    """Test that properties return None when the underlying proto field is unset."""

    @pytest.fixture
    def conn_empty_info(self, connection, mock_db_api):
        """Set up a connection with an empty ConnectionGetInfoResponse."""
        mock_db_api.connection_get_info.return_value = ConnectionGetInfoResponse()
        return connection

    def test_host_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.host is None

    def test_port_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.port is None

    def test_account_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.account is None

    def test_user_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.user is None

    def test_role_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.role is None

    def test_database_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.database is None

    def test_schema_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.schema is None

    def test_warehouse_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.warehouse is None

    def test_session_id_raises_when_unset(self, conn_empty_info):
        with pytest.raises(InterfaceError, match="Session ID is not available"):
            _ = conn_empty_info.session_id


class TestConnectionInfoDelegation:
    """Test that each property delegates to _get_connection_info correctly."""

    def test_each_access_calls_get_connection_info(self, connection, mock_db_api):
        """Each property access should call _get_connection_info (no caching)."""
        mock_db_api.connection_get_info.return_value = ConnectionGetInfoResponse(
            host="h",
            account="a",
            user="u",
            role="r",
            database="d",
            schema="s",
            warehouse="w",
            port=1,
            session_id=1,
        )

        _ = connection.host
        _ = connection.account
        _ = connection.user
        _ = connection.role
        _ = connection.database
        _ = connection.schema
        _ = connection.warehouse
        _ = connection.port
        _ = connection.session_id

        assert mock_db_api.connection_get_info.call_count == 9

    def test_reflects_changing_values(self, connection, mock_db_api):
        """Properties should reflect updated values from sf_core between calls."""
        mock_db_api.connection_get_info.return_value = ConnectionGetInfoResponse(
            database="DB_V1",
            role="ROLE_V1",
        )
        assert connection.database == "DB_V1"
        assert connection.role == "ROLE_V1"

        mock_db_api.connection_get_info.return_value = ConnectionGetInfoResponse(
            database="DB_V2",
            role="ROLE_V2",
        )
        assert connection.database == "DB_V2"
        assert connection.role == "ROLE_V2"


class TestIsStillRunning:
    """Unit tests for Connection.is_still_running."""

    @pytest.mark.parametrize(
        "status, expected",
        [
            (QueryStatus.RUNNING, True),
            (QueryStatus.ABORTING, False),
            (QueryStatus.SUCCESS, False),
            (QueryStatus.FAILED_WITH_ERROR, False),
            (QueryStatus.ABORTED, False),
            (QueryStatus.QUEUED, True),
            (QueryStatus.FAILED_WITH_INCIDENT, False),
            (QueryStatus.DISCONNECTED, False),
            (QueryStatus.RESUMING_WAREHOUSE, True),
            (QueryStatus.QUEUED_REPARING_WAREHOUSE, True),
            (QueryStatus.RESTARTED, False),
            (QueryStatus.BLOCKED, True),
            (QueryStatus.NO_DATA, True),
        ],
    )
    def test_is_still_running(self, status, expected):
        from snowflake.connector.connection import Connection

        assert Connection.is_still_running(status) == expected


class TestIsAnError:
    """Unit tests for Connection.is_an_error."""

    @pytest.mark.parametrize(
        "status, expected",
        [
            (QueryStatus.RUNNING, False),
            (QueryStatus.ABORTING, True),
            (QueryStatus.SUCCESS, False),
            (QueryStatus.FAILED_WITH_ERROR, True),
            (QueryStatus.ABORTED, True),
            (QueryStatus.QUEUED, False),
            (QueryStatus.FAILED_WITH_INCIDENT, True),
            (QueryStatus.DISCONNECTED, True),
            (QueryStatus.RESUMING_WAREHOUSE, False),
            (QueryStatus.QUEUED_REPARING_WAREHOUSE, False),
            (QueryStatus.RESTARTED, False),
            (QueryStatus.BLOCKED, False),
            (QueryStatus.NO_DATA, False),
        ],
    )
    def test_is_an_error(self, status, expected):
        from snowflake.connector.connection import Connection

        assert Connection.is_an_error(status) == expected
