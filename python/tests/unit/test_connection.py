"""
Unit tests for Connection.
"""

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConfigSetting,
    ConnectionGetInfoResponse,
    ConnectionGetQueryStatusResponse,
    ConnectionHandle,
    ConnectionSetOptionsResponse,
    DatabaseHandle,
    ValidationIssue,
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


class TestConnectionSetOptions:
    """Unit tests for the batched connection_set_options RPC during __init__."""

    def test_string_options_use_string_value(self, mock_db_api):
        """String kwargs should be sent as ConfigSetting(string_value=...)."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="alice", account="acme")

        request = mock_db_api.connection_set_options.call_args[0][0]
        assert request.options["user"] == ConfigSetting(string_value="alice")
        assert request.options["account"] == ConfigSetting(string_value="acme")

    def test_int_options_use_int_value(self, mock_db_api):
        """Integer kwargs should be sent as ConfigSetting(int_value=...)."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="u", account="a", port=8080)

        request = mock_db_api.connection_set_options.call_args[0][0]
        assert request.options["port"] == ConfigSetting(int_value=8080)

    def test_bool_options_use_bool_value_not_int(self, mock_db_api):
        """Bool kwargs should use bool_value, not int_value (bool is a subclass of int in Python)."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="u", account="a", insecure_mode=True)

        request = mock_db_api.connection_set_options.call_args[0][0]
        setting = request.options["insecure_mode"]
        assert setting == ConfigSetting(bool_value=True)
        assert setting.WhichOneof("value") == "bool_value"

    def test_float_options_use_double_value(self, mock_db_api):
        """Float kwargs should be sent as ConfigSetting(double_value=...)."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="u", account="a", timeout=30.5)

        request = mock_db_api.connection_set_options.call_args[0][0]
        assert request.options["timeout"] == ConfigSetting(double_value=30.5)

    def test_bytes_options_use_bytes_value(self, mock_db_api):
        """Bytes kwargs should be sent as ConfigSetting(bytes_value=...)."""
        from snowflake.connector.connection import Connection

        token = b"\x01\x02\x03"
        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="u", account="a", token=token)

        request = mock_db_api.connection_set_options.call_args[0][0]
        assert request.options["token"] == ConfigSetting(bytes_value=token)

    def test_all_options_batched_into_single_rpc(self, mock_db_api):
        """All typed options should be submitted in one connection_set_options call."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="u", account="a", port=443, insecure_mode=False, timeout=1.5)

        assert mock_db_api.connection_set_options.call_count == 1
        request = mock_db_api.connection_set_options.call_args[0][0]
        assert set(request.options.keys()) == {"user", "account", "port", "insecure_mode", "timeout", "client_app_id"}

    def test_validation_warnings_forwarded_via_warnings_warn(self, mock_db_api):
        """ValidationIssue warnings from the response should be surfaced via warnings.warn."""
        import warnings

        from snowflake.connector.connection import Connection

        mock_db_api.connection_set_options.return_value = ConnectionSetOptionsResponse(
            warnings=[
                ValidationIssue(message="param 'x' is deprecated"),
                ValidationIssue(message="param 'y' has no effect"),
            ]
        )

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter("always")
                Connection(user="u", account="a")

        assert len(caught) == 2
        assert "param 'x' is deprecated" in str(caught[0].message)
        assert "param 'y' has no effect" in str(caught[1].message)

    def test_no_user_options_sends_only_client_app_id(self, mock_db_api):
        """When there are no user-supplied kwargs, only the injected client_app_id is sent."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(session_parameters={"AUTOCOMMIT": "true"})

        assert mock_db_api.connection_set_options.call_count == 1
        request = mock_db_api.connection_set_options.call_args[0][0]
        assert set(request.options.keys()) == {"client_app_id"}
        assert request.options["client_app_id"] == ConfigSetting(string_value="PythonConnector")


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


class TestApplicationProperty:
    """Unit tests for the Connection.application property."""

    def test_application_defaults_to_python_connector(self, connection):
        assert connection.application == "PythonConnector"

    def test_application_custom_value(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            conn = Connection(user="u", account="a", application="MyApp")
        assert conn.application == "MyApp"

    def test_application_maps_to_client_app_id_option(self, mock_db_api):
        """The application value should be forwarded to sf_core as client_app_id."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            Connection(user="u", account="a", application="CustomApp")

        request = mock_db_api.connection_set_options.call_args[0][0]
        assert request.options["client_app_id"] == ConfigSetting(string_value="CustomApp")
        assert "application" not in request.options

    def test_application_none_defaults_to_client_name(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            conn = Connection(user="u", account="a", application=None)
        assert conn.application == "PythonConnector"

    def test_application_empty_string_defaults_to_client_name(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            conn = Connection(user="u", account="a", application="")
        assert conn.application == "PythonConnector"

    def test_application_rejects_non_string(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            with pytest.raises(ProgrammingError, match="Invalid application parameter"):
                Connection(user="u", account="a", application=123)

    def test_application_rejects_invalid_characters(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            with pytest.raises(ProgrammingError, match="Invalid application name"):
                Connection(user="u", account="a", application="My App!")

    def test_application_not_in_stored_kwargs(self, mock_db_api):
        """application should be popped from kwargs so it doesn't leak into stored kwargs."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
            conn = Connection(user="u", account="a", application="MyApp")
        assert "application" not in conn.kwargs


class TestConnectionArrowProperties:
    """Unit tests for Connection properties (getters/setters)."""

    def test_arrow_number_to_decimal_default_is_false(self, connection):
        assert connection.arrow_number_to_decimal is False

    def test_arrow_number_to_decimal_setter_enables(self, connection):
        connection.arrow_number_to_decimal = True
        assert connection.arrow_number_to_decimal is True

    def test_arrow_number_to_decimal_setter_enables_backward_compatible(self, connection):
        connection.arrow_number_to_decimal_setter = True
        assert connection.arrow_number_to_decimal is True

    def test_arrow_number_to_decimal_setter_disables(self, connection):
        connection.arrow_number_to_decimal = True
        connection.arrow_number_to_decimal = False
        assert connection.arrow_number_to_decimal is False

    def test_arrow_number_to_decimal_setter_coerces_to_bool(self, connection):
        connection.arrow_number_to_decimal = 1
        assert connection.arrow_number_to_decimal is True

        connection.arrow_number_to_decimal = 0
        assert connection.arrow_number_to_decimal is False


class TestGetQueryStatus:
    """Unit tests for Connection.get_query_status."""

    @pytest.mark.parametrize(
        "status_name, expected",
        [
            ("SUCCESS", QueryStatus.SUCCESS),
            ("RUNNING", QueryStatus.RUNNING),
            ("FAILED_WITH_ERROR", QueryStatus.FAILED_WITH_ERROR),
            ("QUEUED", QueryStatus.QUEUED),
            ("ABORTING", QueryStatus.ABORTING),
            ("ABORTED", QueryStatus.ABORTED),
            ("RESUMING_WAREHOUSE", QueryStatus.RESUMING_WAREHOUSE),
            ("QUEUED_REPARING_WAREHOUSE", QueryStatus.QUEUED_REPARING_WAREHOUSE),
            ("FAILED_WITH_INCIDENT", QueryStatus.FAILED_WITH_INCIDENT),
            ("DISCONNECTED", QueryStatus.DISCONNECTED),
            ("RESTARTED", QueryStatus.RESTARTED),
            ("BLOCKED", QueryStatus.BLOCKED),
            ("NO_DATA", QueryStatus.NO_DATA),
        ],
    )
    def test_maps_status_name_to_enum(self, connection, mock_db_api, status_name, expected):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name=status_name,
        )
        assert connection.get_query_status("test-query-id") == expected

    def test_unknown_status_returns_no_data(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name="SOME_FUTURE_STATUS",
        )
        assert connection.get_query_status("test-query-id") == QueryStatus.NO_DATA

    def test_passes_correct_conn_handle_and_query_id(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name="SUCCESS",
        )
        connection.get_query_status("abc-123")

        args, _ = mock_db_api.connection_get_query_status.call_args
        request = args[0]
        assert request.conn_handle == connection.conn_handle
        assert request.query_id == "abc-123"

    def test_propagates_proto_error(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.side_effect = ProgrammingError("Query not found")
        with pytest.raises(ProgrammingError, match="Query not found"):
            connection.get_query_status("invalid-id")


class TestGetQueryStatusThrowIfError:
    """Unit tests for Connection.get_query_status_throw_if_error."""

    def test_returns_status_on_success(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name="SUCCESS",
        )
        assert connection.get_query_status_throw_if_error("qid") == QueryStatus.SUCCESS

    def test_returns_status_when_running(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name="RUNNING",
        )
        assert connection.get_query_status_throw_if_error("qid") == QueryStatus.RUNNING

    def test_raises_on_error_status_with_details(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name="FAILED_WITH_ERROR",
            error_code=1003,
            error_message="SQL compilation error",
        )
        with pytest.raises(ProgrammingError, match="SQL compilation error") as exc_info:
            connection.get_query_status_throw_if_error("failed-qid")
        assert exc_info.value.errno == 1003
        assert exc_info.value.sfqid == "failed-qid"

    def test_raises_on_aborted_status(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name="ABORTED",
        )
        with pytest.raises(ProgrammingError) as exc_info:
            connection.get_query_status_throw_if_error("aborted-qid")
        assert exc_info.value.sfqid == "aborted-qid"

    def test_raises_with_fallback_message_when_no_error_message(self, connection, mock_db_api):
        mock_db_api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
            status_name="FAILED_WITH_ERROR",
        )
        with pytest.raises(ProgrammingError, match="Query failed-qid-2 failed"):
            connection.get_query_status_throw_if_error("failed-qid-2")
