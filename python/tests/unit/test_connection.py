"""
Unit tests for Connection.
"""

import logging
import warnings

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.binding_converters import ParamStyle
from snowflake.connector._internal.connection import CURRENT_VERSION_SQL
from snowflake.connector._internal.errorcode import ER_INVALID_VALUE, ER_INVALID_WIF_SETTINGS
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    VALIDATION_CODE_CONFLICTING_PARAMETERS,
    VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS,
    ConfigSetting,
    ConnectionGetInfoResponse,
    ConnectionGetQueryStatusResponse,
    ConnectionIsClosedResponse,
    ConnectionIsExpiredResponse,
    ConnectionSetOptionsResponse,
    StatementHandle,
    ValidationIssue,
)
from snowflake.connector.connection import Connection
from snowflake.connector.constants import QueryStatus
from snowflake.connector.cursor import SnowflakeCursor
from snowflake.connector.errors import InterfaceError, ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


@pytest.fixture
def connection(mock_db_api):
    """Create a Connection with core_driver patched via mock_db_api fixture."""
    conn = Connection(user="test_user", account="test_account")
    yield conn
    # Prevent a late __del__ (deferred GC, especially on Python 3.14+) from
    # calling close() through the *next* test's mock_db_api and polluting its
    # call counts.
    conn.auto_cleanup = False


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
        with pytest.raises(ProgrammingError, match="Invalid autocommit parameter"):
            Connection(user="test_user", account="test_account", autocommit=1)


class TestParamstyleSetter:
    """PEP 249 uses string paramstyle; assignment normalizes once on the connection."""

    @pytest.fixture(autouse=True)
    def _no_native_stream_ops(self):
        """Avoid touching real Arrow stream memory when cursor tests run execute()."""
        with (
            patch("snowflake.connector._internal.cursor.query_result.get_stream_ptr", return_value=0),
            patch("snowflake.connector._internal.cursor.query_result.release_arrow_stream"),
        ):
            yield

    def test_assign_string_normalizes(self, connection):
        connection.paramstyle = "qmark"
        assert connection.paramstyle == ParamStyle.QMARK
        assert connection._paramstyle is ParamStyle.QMARK
        connection.paramstyle = "  QMARK  "
        assert connection.paramstyle == ParamStyle.QMARK

    def test_assign_via_private_paramstyle_normalizes(self, connection):
        """Legacy / SnowPy code sets ``conn._paramstyle`` directly; must coerce like ``paramstyle``."""
        connection._paramstyle = "numeric"
        assert connection.paramstyle == ParamStyle.NUMERIC
        assert connection._paramstyle == ParamStyle.NUMERIC

    def test_assign_enum_unchanged(self, connection):
        connection.paramstyle = ParamStyle.NUMERIC
        assert connection.paramstyle == ParamStyle.NUMERIC

    def test_assign_invalid_string_raises(self, connection):
        with pytest.raises(ProgrammingError, match="Invalid paramstyle"):
            connection.paramstyle = "bogus"

    def test_assign_invalid_type_raises(self, connection):
        with pytest.raises(ProgrammingError, match="paramstyle must be str or ParamStyle"):
            connection.paramstyle = 123  # type: ignore[assignment]

    def test_cursor_execute_qmark_after_string_assign(self, connection, mock_db_api):
        mock_db_api.statement_new.return_value = MagicMock(stmt_handle=StatementHandle(id=1))
        execute_result = MagicMock()
        execute_result.columns = []
        execute_result.HasField = MagicMock(return_value=False)
        execute_result.sql_state = "00000"
        mock_db_api.statement_execute_query.return_value.result = execute_result

        connection.paramstyle = "qmark"
        cur = SnowflakeCursor(connection)
        cur.execute("SELECT ?", (1,))
        req = mock_db_api.statement_execute_query.call_args[0][0]
        assert req.HasField("bindings")


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

    def test_get_autocommit_reads_from_sf_core(self, connection, mock_db_api):
        """get_autocommit should read from sf_core via _session_parameters."""
        assert connection.get_autocommit() is False
        mock_db_api.connection_get_parameter.return_value = MagicMock(value="true")
        assert connection.get_autocommit() is True


class TestTelemetryEnabled:
    """Unit tests for Connection.telemetry_enabled."""

    def test_false_when_server_param_unset(self, connection):
        """Absent CLIENT_TELEMETRY_ENABLED session parameter means unconfirmed, matching legacy."""
        assert connection.telemetry_enabled is False

    @pytest.mark.parametrize(
        ("server_value", "expected"),
        [
            ("true", True),
            ("TRUE", True),
            ("True", True),
            ("false", False),
            ("yes", False),
        ],
    )
    def test_reads_server_param_case_insensitively(self, connection, mock_db_api, server_value, expected):
        mock_db_api.connection_get_parameter.return_value = MagicMock(value=server_value)
        assert connection.telemetry_enabled is expected

    def test_client_false_overrides_server_true(self, connection, mock_db_api):
        mock_db_api.connection_get_parameter.return_value = MagicMock(value="true")
        connection.telemetry_enabled = False
        assert connection.telemetry_enabled is False

    def test_client_default_and_server_true(self, connection, mock_db_api):
        mock_db_api.connection_get_parameter.return_value = MagicMock(value="true")
        assert connection.telemetry_enabled is True

    @pytest.mark.parametrize("raw_value", [1, 0, "", None])
    def test_setter_coerces_to_bool(self, connection, raw_value):
        connection.telemetry_enabled = raw_value
        assert connection._client_param_telemetry_enabled is (bool(raw_value))
        assert isinstance(connection._client_param_telemetry_enabled, bool)

    def test_enabling_while_server_disabled_logs_info(self, connection, caplog):
        """Re-enabling while the server half is off should log the legacy message."""
        with caplog.at_level(logging.INFO):
            connection.telemetry_enabled = True
        assert "Telemetry has been disabled by the session parameter CLIENT_TELEMETRY_ENABLED" in caplog.text

    def test_enabling_while_server_enabled_does_not_log(self, connection, mock_db_api, caplog):
        mock_db_api.connection_get_parameter.return_value = MagicMock(value="true")
        with caplog.at_level(logging.INFO):
            connection.telemetry_enabled = True
        assert "CLIENT_TELEMETRY_ENABLED" not in caplog.text

    def test_disabling_does_not_log(self, connection, caplog):
        with caplog.at_level(logging.INFO):
            connection.telemetry_enabled = False
        assert "CLIENT_TELEMETRY_ENABLED" not in caplog.text

    def test_getter_never_raises_on_rpc_failure(self, connection, mock_db_api):
        mock_db_api.connection_get_parameter.side_effect = RuntimeError("boom")
        assert connection.telemetry_enabled is False

    def test_read_after_close_uses_frozen_snapshot(self, connection, mock_db_api):
        """Post-close reads should answer from the frozen snapshot, matching legacy retaining its last value."""
        mock_db_api.connection_get_all_parameters.return_value = MagicMock(
            parameters={"CLIENT_TELEMETRY_ENABLED": "true"}
        )
        connection.close()

        assert connection.telemetry_enabled is True
        call_count_before = mock_db_api.connection_get_parameter.call_count
        assert connection.telemetry_enabled is True
        assert mock_db_api.connection_get_parameter.call_count == call_count_before


class TestAutocommitKwargUnit:
    """Unit tests for the autocommit keyword argument at connection time."""

    def test_autocommit_true_injects_session_parameter(self, mock_db_api):
        """Connection(autocommit=True) should pass AUTOCOMMIT=true as a session parameter."""
        from snowflake.connector.connection import Connection

        Connection(user="test_user", account="test_account", autocommit=True)

        call_args = mock_db_api.connection_set_session_parameters.call_args
        params = call_args[0][0].parameters
        assert params["AUTOCOMMIT"] == "true"

    def test_autocommit_false_injects_session_parameter(self, mock_db_api):
        """Connection(autocommit=False) should pass AUTOCOMMIT=false as a session parameter."""
        from snowflake.connector.connection import Connection

        Connection(user="test_user", account="test_account", autocommit=False)

        call_args = mock_db_api.connection_set_session_parameters.call_args
        params = call_args[0][0].parameters
        assert params["AUTOCOMMIT"] == "false"

    def test_no_autocommit_kwarg_does_not_set_autocommit(self, mock_db_api):
        """Connection without autocommit kwarg should not inject AUTOCOMMIT, preserving server default."""
        from snowflake.connector.connection import Connection

        Connection(user="test_user", account="test_account")

        call_args = mock_db_api.connection_set_session_parameters.call_args
        if call_args is not None:
            params = call_args[0][0].parameters
            assert "AUTOCOMMIT" not in params
        # If connection_set_session_parameters was not called at all, that's also correct


class TestClientSessionKeepAliveKwargUnit:
    """Unit tests for client_session_keep_alive[_heartbeat_frequency] kwargs."""

    def test_keep_alive_kwargs_forwarded_to_set_options(self, mock_db_api):
        from snowflake.connector.connection import Connection

        Connection(
            user="u",
            account="a",
            client_session_keep_alive=True,
            client_session_keep_alive_heartbeat_frequency=1500,
        )

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["CLIENT_SESSION_KEEP_ALIVE"] == ConfigSetting(bool_value=True)
        assert request.options["CLIENT_SESSION_KEEP_ALIVE_HEARTBEAT_FREQUENCY"] == ConfigSetting(int_value=1500)

    def test_keep_alive_properties_read_from_config(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(
            user="u",
            account="a",
            client_session_keep_alive=True,
            client_session_keep_alive_heartbeat_frequency=900,
        )

        assert conn.client_session_keep_alive is True
        assert conn.client_session_keep_alive_heartbeat_frequency == 900

    def test_keep_alive_properties_defaults(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a")

        assert conn.client_session_keep_alive is False
        assert conn.client_session_keep_alive_heartbeat_frequency is None

    def test_keep_alive_attributes_are_read_only(self, connection):
        # The old Python connector exposed setters but they were no-ops on
        # the live heartbeat thread. The Universal Driver drops them so an
        # accidental post-connect assignment fails loudly instead of being
        # silently ignored.
        with pytest.raises(AttributeError):
            connection.client_session_keep_alive = True
        with pytest.raises(AttributeError):
            connection.client_session_keep_alive_heartbeat_frequency = 600


class TestTimeoutPropertiesUnit:
    """Unit tests for login_timeout / network_timeout / socket_timeout properties."""

    def test_login_timeout_read_from_config(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", login_timeout=45)

        assert conn.login_timeout == 45

    def test_login_timeout_default(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a")

        assert conn.login_timeout == 120

    def test_network_timeout_fans_out_to_query_and_request_timeout(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with pytest.warns(DeprecationWarning, match="network_timeout"):
            conn = Connection(user="u", account="a", network_timeout=45)

        assert conn.network_timeout == 45

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["query_timeout"] == ConfigSetting(int_value=45)
        assert request.options["request_timeout"] == ConfigSetting(int_value=45)

    def test_socket_timeout_fans_out_to_connect_and_retry_timeout(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with pytest.warns(DeprecationWarning, match="socket_timeout"):
            conn = Connection(user="u", account="a", socket_timeout=30)

        assert conn.socket_timeout == 30

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["connect_timeout"] == ConfigSetting(int_value=30)
        assert request.options["retry_timeout"] == ConfigSetting(int_value=30)

    def test_network_and_socket_timeout_defaults(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a")

        assert conn.network_timeout == 120
        assert conn.socket_timeout is None

    def test_network_timeout_getter_reflects_explicit_request_timeout(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with pytest.warns(DeprecationWarning, match="network_timeout"):
            conn = Connection(user="u", account="a", network_timeout=45, request_timeout=60)

        assert conn.network_timeout == 60


class TestConnectionSetOptions:
    """Unit tests for the batched connection_set_options RPC during __init__."""

    def test_string_options_use_string_value(self, mock_db_api):
        """String kwargs should be sent as ConfigSetting(string_value=...)."""
        from snowflake.connector.connection import Connection

        Connection(user="alice", account="acme")

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["user"] == ConfigSetting(string_value="alice")
        assert request.options["account"] == ConfigSetting(string_value="acme")

    def test_int_options_use_int_value(self, mock_db_api):
        """Integer kwargs should be sent as ConfigSetting(int_value=...)."""
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", port=8080)

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["port"] == ConfigSetting(int_value=8080)

    def test_bool_options_use_bool_value_not_int(self, mock_db_api):
        """Bool kwargs should use bool_value, not int_value (bool is a subclass of int in Python)."""
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", insecure_mode=True)

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        setting = request.options["insecure_mode"]
        assert setting == ConfigSetting(bool_value=True)
        assert setting.WhichOneof("value") == "bool_value"

    def test_float_options_use_double_value(self, mock_db_api):
        """Float kwargs should be sent as ConfigSetting(double_value=...)."""
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", timeout=30.5)

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["timeout"] == ConfigSetting(double_value=30.5)

    def test_bytes_options_use_bytes_value(self, mock_db_api):
        """Bytes kwargs should be sent as ConfigSetting(bytes_value=...)."""
        from snowflake.connector.connection import Connection

        token = b"\x01\x02\x03"
        Connection(user="u", account="a", token=token)

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["token"] == ConfigSetting(bytes_value=token)

    def test_all_options_batched_into_single_rpc(self, mock_db_api):
        """All typed options should be submitted in one connection_set_options call."""
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", port=443, insecure_mode=False, timeout=1.5)

        # Single batched call: generic kwargs + logout config combined
        assert mock_db_api.connection_set_options.call_count == 1
        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        # Generic kwargs + logout config keys in one batch
        assert {"user", "account", "port", "insecure_mode", "timeout", "client_app_id"}.issubset(
            set(request.options.keys())
        )

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

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            Connection(user="u", account="a")

        # Filter out FutureWarning from _extract_auto_detection_param
        validation_warnings = [w for w in caught if w.category is not FutureWarning]
        assert len(validation_warnings) == 2
        assert "param 'x' is deprecated" in str(validation_warnings[0].message)
        assert "param 'y' has no effect" in str(validation_warnings[1].message)

    def test_no_user_options_sends_client_app_id_and_logout_defaults(self, mock_db_api):
        """When there are no user-supplied kwargs, client_app_id, application,
        and logout defaults are sent."""
        from snowflake.connector.connection import Connection

        Connection(session_parameters={"AUTOCOMMIT": "true"})

        # Single batched call: client_app_id + application + logout config defaults
        assert mock_db_api.connection_set_options.call_count == 1
        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert "client_app_id" in request.options
        assert request.options["client_app_id"] == ConfigSetting(string_value="PythonConnector")
        assert "application" in request.options
        assert request.options["application"] == ConfigSetting(string_value="PythonConnector")


class TestDriverIdentity:
    """Unit tests for driver identity fields in ConnectionInitRequest."""

    def test_driver_identity_in_connection_init(self, mock_db_api):
        """Driver identity fields should be passed via WrapperIdentity in ConnectionInitRequest."""
        import platform

        from snowflake.connector.connection import Connection
        from snowflake.connector.version import __version__

        Connection(user="u", account="a")

        init_request = mock_db_api.connection_init.call_args[0][0]
        identity = init_request.wrapper_identity
        assert identity.driver_name == "PythonConnector"
        assert identity.driver_version == __version__
        assert identity.language_runtime == platform.python_implementation()
        assert identity.language_version == platform.python_version()
        assert identity.language_compiler == platform.python_compiler()


class TestWifConflictErrnoRemap:
    """Unit tests for the WIF cross-param errno remap in Connection._connect().

    ``connection_init`` surfaces sf_core's WIF cross-param validation failures as a
    ``ProgrammingError`` with ``errno=ER_INVALID_VALUE`` and structured ``parameter``/
    ``validation_code`` attributes set by the lower conversion layer. ``_connect()``
    re-raises with ``errno=ER_INVALID_WIF_SETTINGS`` for legacy parity, and must forward
    the same ``parameter``/``validation_code`` onto the re-raised exception rather than
    dropping them.
    """

    def test_remapped_exception_carries_parameter_and_validation_code(self, mock_db_api):
        """The remapped ProgrammingError should carry the original parameter/validation_code."""
        mock_db_api.connection_init.side_effect = ProgrammingError(
            msg="workload_identity_provider was set but authenticator was not WORKLOAD_IDENTITY",
            errno=ER_INVALID_VALUE,
            parameter="workload_identity_provider",
            validation_code=VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS,
        )

        with pytest.raises(ProgrammingError) as excinfo:
            Connection(user="test_user", account="test_account")

        assert excinfo.value.errno == ER_INVALID_WIF_SETTINGS
        assert excinfo.value.parameter == "workload_identity_provider"
        assert excinfo.value.validation_code == VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS

    def test_non_wif_programming_error_is_reraised_unchanged(self, mock_db_api):
        """A generic CONFLICTING_PARAMETERS error (not WIF) should pass through as-is."""
        mock_db_api.connection_init.side_effect = ProgrammingError(
            msg="Both 'private_key' and 'private_key_file' are set. Please provide only one.",
            errno=ER_INVALID_VALUE,
            parameter="private_key",
            validation_code=VALIDATION_CODE_CONFLICTING_PARAMETERS,
        )

        with pytest.raises(ProgrammingError) as excinfo:
            Connection(user="test_user", account="test_account")

        assert excinfo.value.errno == ER_INVALID_VALUE
        assert excinfo.value.parameter == "private_key"


class TestAsyncWifConflictErrnoRemap:
    """Unit tests for the WIF cross-param errno remap in the async Connection.connect()."""

    def test_remapped_exception_carries_parameter_and_validation_code(self, mock_async_db_api):
        # reference-driver: local import avoids collection-time ImportError — the
        # reference driver has no `snowflake.connector.aio`, and pytest.mark.skipif
        # only skips execution, not module import/collection.
        import asyncio

        from snowflake.connector.aio.connection._connection import Connection as AsyncConnection

        mock_async_db_api.connection_init.side_effect = ProgrammingError(
            msg="workload_identity_provider was set but authenticator was not WORKLOAD_IDENTITY",
            errno=ER_INVALID_VALUE,
            parameter="workload_identity_provider",
            validation_code=VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS,
        )

        conn = AsyncConnection(user="test_user", account="test_account")
        with pytest.raises(ProgrammingError) as excinfo:
            asyncio.run(conn.connect())

        assert excinfo.value.errno == ER_INVALID_WIF_SETTINGS
        assert excinfo.value.parameter == "workload_identity_provider"
        assert excinfo.value.validation_code == VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS

    def test_non_wif_programming_error_is_reraised_unchanged(self, mock_async_db_api):
        """A generic CONFLICTING_PARAMETERS error (not WIF) should pass through as-is."""
        # reference-driver: local import avoids collection-time ImportError — the
        # reference driver has no `snowflake.connector.aio`, and pytest.mark.skipif
        # only skips execution, not module import/collection.
        import asyncio

        from snowflake.connector.aio.connection._connection import Connection as AsyncConnection

        mock_async_db_api.connection_init.side_effect = ProgrammingError(
            msg="Both 'private_key' and 'private_key_file' are set. Please provide only one.",
            errno=ER_INVALID_VALUE,
            parameter="private_key",
            validation_code=VALIDATION_CODE_CONFLICTING_PARAMETERS,
        )

        conn = AsyncConnection(user="test_user", account="test_account")
        with pytest.raises(ProgrammingError) as excinfo:
            asyncio.run(conn.connect())

        assert excinfo.value.errno == ER_INVALID_VALUE
        assert excinfo.value.parameter == "private_key"


class TestClose:
    """Unit tests for Connection.close() and handle release."""

    def test_close_sends_connection_close_and_releases_handles(self, connection, mock_db_api):
        """close() should send connection_close RPC then release both handles."""
        connection.close()

        mock_db_api.connection_close.assert_called_once()
        mock_db_api.connection_release.assert_called_once()
        mock_db_api.database_release.assert_called_once()

    def test_close_nullifies_handles(self, connection, mock_db_api):
        """close() should set conn_handle and db_handle to None to prevent use-after-release."""
        assert connection.conn_handle is not None
        assert connection.db_handle is not None

        connection.close()

        assert connection.conn_handle is None
        assert connection.db_handle is None

    def test_close_is_idempotent(self, connection, mock_db_api):
        """Calling close() multiple times should only close and release once."""
        # First is_closed() returns False (close proceeds), subsequent return True (idempotent)
        mock_db_api.connection_is_closed.side_effect = [
            ConnectionIsClosedResponse(is_closed=False),
            ConnectionIsClosedResponse(is_closed=True),
            ConnectionIsClosedResponse(is_closed=True),
            ConnectionIsClosedResponse(is_closed=True),
        ]
        connection.close()
        connection.close()
        connection.close()

        mock_db_api.connection_close.assert_called_once()
        mock_db_api.connection_release.assert_called_once()
        mock_db_api.database_release.assert_called_once()

    def test_close_releases_database_even_if_connection_release_fails(self, connection, mock_db_api):
        """database_release should still be called if connection_release raises."""
        mock_db_api.connection_release.side_effect = RuntimeError("release failed")

        connection.close()

        mock_db_api.connection_close.assert_called_once()
        mock_db_api.connection_release.assert_called_once()
        mock_db_api.database_release.assert_called_once()

    def test_del_releases_handles_if_not_closed(self, mock_db_api):
        """__del__ should close + release handles when close() was never called."""
        from snowflake.connector.connection import Connection

        conn = Connection(user="test_user", account="test_account")

        conn.__del__()
        conn.auto_cleanup = False

        mock_db_api.connection_close.assert_called_once()
        mock_db_api.connection_release.assert_called_once()
        mock_db_api.database_release.assert_called_once()

    def test_del_does_not_raise(self, connection, mock_db_api):
        """__del__ must never propagate exceptions (best-effort via _try_close)."""
        mock_db_api.connection_release.side_effect = RuntimeError("boom")
        mock_db_api.database_release.side_effect = RuntimeError("boom")

        # Should not raise
        connection.__del__()

    def test_del_is_noop_if_already_closed(self, connection, mock_db_api):
        """__del__ should not close again if already closed."""
        # After close(), is_closed() returns True
        mock_db_api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=True)

        connection.__del__()

        mock_db_api.connection_close.assert_not_called()


class TestContextManagerUnit:
    """Unit tests for __exit__ behavior."""

    def test_exit_skips_commit_when_autocommit_on(self, connection, mock_db_api):
        """When autocommit is on, __exit__ should not execute COMMIT or ROLLBACK."""
        mock_db_api.connection_get_parameter.return_value = MagicMock(value="true")
        connection.commit = MagicMock()
        connection.rollback = MagicMock()

        connection.__exit__(None, None, None)

        connection.commit.assert_not_called()
        connection.rollback.assert_not_called()

    def test_exit_always_closes(self, connection, mock_db_api):
        """close() should be called even if commit raises an exception."""

        def failing_commit():
            raise RuntimeError("commit failed")

        connection.commit = failing_commit

        with pytest.raises(RuntimeError, match="commit failed"):
            connection.__exit__(None, None, None)

        # Verify Core's connection_close RPC was invoked (close() was called in finally block)
        mock_db_api.connection_close.assert_called_once()

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
            proxy_host="proxy.example.com",
            proxy_port=8080,
            proxy_user="puser",
            proxy_password="ppass",
            no_proxy="localhost,127.0.0.1",
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

    def test_proxy_host_returns_value(self, conn_with_info):
        assert conn_with_info.proxy_host == "proxy.example.com"

    def test_proxy_port_returns_value(self, conn_with_info):
        assert conn_with_info.proxy_port == 8080

    def test_proxy_user_returns_value(self, conn_with_info):
        assert conn_with_info.proxy_user == "puser"

    def test_proxy_password_returns_value(self, conn_with_info):
        assert conn_with_info.proxy_password == "ppass"

    def test_no_proxy_returns_value(self, conn_with_info):
        assert conn_with_info.no_proxy == "localhost,127.0.0.1"


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

    def test_proxy_host_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.proxy_host is None

    def test_proxy_port_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.proxy_port is None

    def test_proxy_user_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.proxy_user is None

    def test_proxy_password_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.proxy_password is None

    def test_no_proxy_none_when_unset(self, conn_empty_info):
        assert conn_empty_info.no_proxy is None


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


class TestSnowflakeVersionProperty:
    """Unit tests for the Connection.snowflake_version cached property."""

    def test_returns_version_string(self, connection):
        mock_cursor = MagicMock()
        mock_cursor.__enter__ = MagicMock(return_value=mock_cursor)
        mock_cursor.__exit__ = MagicMock(return_value=False)
        mock_cursor.fetchone.return_value = {"VERSION": "8.46.1"}
        connection.cursor = MagicMock(return_value=mock_cursor)

        assert connection.snowflake_version == "8.46.1"
        mock_cursor.execute.assert_called_once_with(CURRENT_VERSION_SQL)

    def test_strips_suffix_after_space(self, connection):
        """The legacy driver splits on space and takes the first part."""
        mock_cursor = MagicMock()
        mock_cursor.__enter__ = MagicMock(return_value=mock_cursor)
        mock_cursor.__exit__ = MagicMock(return_value=False)
        mock_cursor.fetchone.return_value = {"VERSION": "8.46.1 some extra info"}
        connection.cursor = MagicMock(return_value=mock_cursor)

        assert connection.snowflake_version == "8.46.1"

    def test_result_is_cached(self, connection):
        mock_cursor = MagicMock()
        mock_cursor.__enter__ = MagicMock(return_value=mock_cursor)
        mock_cursor.__exit__ = MagicMock(return_value=False)
        mock_cursor.fetchone.return_value = {"VERSION": "8.46.1"}
        connection.cursor = MagicMock(return_value=mock_cursor)

        _ = connection.snowflake_version
        _ = connection.snowflake_version

        mock_cursor.execute.assert_called_once()


class TestApplicationProperty:
    """Unit tests for the Connection.application property."""

    def test_application_defaults_to_python_connector(self, connection):
        assert connection.application == "PythonConnector"

    def test_application_custom_value(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", application="MyApp")
        assert conn.application == "MyApp"

    def test_application_maps_to_application_option(self, mock_db_api):
        """The user's application value goes to the canonical ``application``
        setting (CLIENT_ENVIRONMENT.APPLICATION on the wire), while
        client_app_id (CLIENT_APP_ID) stays as the driver name."""
        from snowflake.connector.connection import CLIENT_NAME, Connection

        Connection(user="u", account="a", application="CustomApp")

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["client_app_id"] == ConfigSetting(string_value=CLIENT_NAME)
        assert request.options["application"] == ConfigSetting(string_value="CustomApp")

    def test_custom_application_does_not_override_client_app_id(self, mock_db_api):
        """In the old connector, setting application='SNOWCLI.STAGE.COPY' only
        affects CLIENT_ENVIRONMENT.APPLICATION. CLIENT_APP_ID always comes from
        ``internal_application_name`` (defaults to 'PythonConnector'). The new
        connector must preserve this separation so that server-side feature
        gating tied to the client type continues to work.
        """
        from snowflake.connector.connection import CLIENT_NAME, Connection

        Connection(user="u", account="a", application="SNOWCLI.STAGE.COPY")

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["client_app_id"] == ConfigSetting(string_value=CLIENT_NAME)
        assert request.options["application"] == ConfigSetting(string_value="SNOWCLI.STAGE.COPY")

    def test_application_none_defaults_to_client_name(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", application=None)
        assert conn.application == "PythonConnector"

    def test_application_empty_string_defaults_to_client_name(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", application="")
        assert conn.application == "PythonConnector"

    def test_application_accepts_dotted_name(self, mock_db_api):
        """Snow CLI passes dotted names like 'SNOWCLI.STAGE.COPY'."""
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", application="SNOWCLI.STAGE.COPY")
        assert conn.application == "SNOWCLI.STAGE.COPY"

    def test_application_rejects_non_string(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with pytest.raises(ProgrammingError, match="Invalid application parameter"):
            Connection(user="u", account="a", application=123)

    def test_application_rejects_name_starting_with_non_word_char(self, mock_db_api):
        from snowflake.connector.connection import Connection

        with pytest.raises(ProgrammingError, match="Invalid application name"):
            Connection(user="u", account="a", application="!invalid")

    def test_application_stored_in_config(self, mock_db_api):
        """application should be stored in config, not leaked into kwargs."""
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", application="MyApp")
        assert conn.config.application == "MyApp"


class TestInternalApplicationName:
    """Unit tests for internal_application_name and internal_application_version kwargs.

    Mirrors the old connector where ``internal_application_name`` /
    ``internal_application_version`` override ``CLIENT_APP_ID`` /
    ``CLIENT_APP_VERSION`` in the login request. Tools like SnowSQL and Snow
    CLI use these to identify themselves to the server.
    """

    def test_internal_application_name_overrides_client_app_id(self, mock_db_api):
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", internal_application_name="SnowSQL")

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["client_app_id"] == ConfigSetting(string_value="SnowSQL")

    def test_internal_application_name_defaults_to_client_name(self, mock_db_api):
        from snowflake.connector.connection import CLIENT_NAME, Connection

        Connection(user="u", account="a")

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["client_app_id"] == ConfigSetting(string_value=CLIENT_NAME)

    def test_internal_application_name_does_not_affect_application_property(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", internal_application_name="SnowSQL")
        assert conn.application == "PythonConnector"

    def test_internal_application_name_combined_with_application(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(
            user="u",
            account="a",
            internal_application_name="SnowSQL",
            application="SNOWCLI.STAGE.COPY",
        )

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["client_app_id"] == ConfigSetting(string_value="SnowSQL")
        assert request.options["application"] == ConfigSetting(string_value="SNOWCLI.STAGE.COPY")
        assert conn.application == "SNOWCLI.STAGE.COPY"

    def test_internal_application_version_overrides_client_app_version(self, mock_db_api):
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", internal_application_version="1.2.3")

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["client_app_version"] == ConfigSetting(string_value="1.2.3")

    def test_internal_application_version_defaults_to_driver_version(self, mock_db_api):
        """When the caller does not override, client_app_version falls back to
        the Python driver's own __version__ — matching the old connector."""
        from snowflake.connector.connection import Connection
        from snowflake.connector.version import __version__

        Connection(user="u", account="a")

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["client_app_version"] == ConfigSetting(string_value=__version__)


class TestLogMaxQueryLength:
    """Unit tests for Connection.log_max_query_length and _format_query_for_log."""

    def test_default_value_is_80(self, connection):
        assert connection.log_max_query_length == 80

    def test_custom_value_at_connect_time(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", log_max_query_length=200)
        assert conn.log_max_query_length == 200

    def test_format_short_query_unchanged(self, connection):
        query = "SELECT 1"
        assert connection._format_query_for_log(query) == "SELECT 1"

    def test_format_long_query_truncated(self, connection):
        query = "x" * 100
        result = connection._format_query_for_log(query)
        assert result == "x" * 80 + "..."
        assert len(result) == 83

    def test_format_query_one_below_boundary(self, connection):
        query = "x" * 79
        assert connection._format_query_for_log(query) == "x" * 79

    def test_format_collapses_newlines(self, connection):
        query = "SELECT\n    col1,\n    col2\nFROM\n    my_table"
        result = connection._format_query_for_log(query)
        assert "\n" not in result
        assert result == "SELECT col1, col2 FROM my_table"

    def test_format_strips_leading_trailing_whitespace_per_line(self, connection):
        query = "  SELECT 1  \n  FROM dual  "
        result = connection._format_query_for_log(query)
        assert result == "SELECT 1 FROM dual"

    def test_format_collapses_then_truncates(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", log_max_query_length=20)

        query = "SELECT\n    very_long_column_name\nFROM\n    my_table"
        result = conn._format_query_for_log(query)
        assert len(result) == 23  # 20 + "..."
        assert result.endswith("...")

    def test_custom_zero_truncates_everything(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", log_max_query_length=0)

        assert conn._format_query_for_log("SELECT 1") == "..."

    def test_sent_to_sf_core(self, mock_db_api):
        """log_max_query_length is used by both Python (log formatting) and Core (log truncation)."""
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", log_max_query_length=200)

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert request.options["log_max_query_length"] == ConfigSetting(int_value=200)

    def test_format_query_at_exact_boundary(self, connection):
        """Legacy uses strict less-than: a query of exactly log_max_query_length chars IS truncated."""
        query = "x" * 80
        result = connection._format_query_for_log(query)
        assert result == "x" * 80 + "..."


class TestConnectionArrowProperties:
    """Unit tests for Connection properties (getters/setters)."""

    def test_arrow_number_to_decimal_default_is_false(self, connection):
        # Regression: arrow_number_to_decimal initialization was previously in dead
        # code after a ``return`` statement inside ``_map_logout_config()``, making it
        # unreachable during ``__init__``.  Reading the property right after init
        # would AttributeError if the underlying ``config`` value is not set, so this
        # test also guards against that regression.
        assert connection.arrow_number_to_decimal is False

    def test_arrow_number_to_decimal_setter_enables(self, connection):
        connection.arrow_number_to_decimal = True
        assert connection.arrow_number_to_decimal is True

    def test_arrow_number_to_decimal_setter_enables_backward_compatible(self, connection):
        connection.arrow_number_to_decimal_setter = True
        assert connection.arrow_number_to_decimal is True

    def test_arrow_number_to_decimal_setter_emits_deprecation_warning_once(self, connection):
        """The legacy ``arrow_number_to_decimal_setter`` alias is decorated
        with ``@backward_compatibility``; assigning to it from external code
        must emit a ``DeprecationWarning`` exactly once per process."""
        from snowflake.connector._internal.backward_compatibility import _BACKWARD_COMPAT_WARNED

        # Snapshot/restore just this one dedup slot so the test is order-
        # independent without leaking state across the session.
        key = ("snowflake.connector._internal.connection.connection", "ConnectionMixin.arrow_number_to_decimal_setter")
        was_warned = key in _BACKWARD_COMPAT_WARNED
        _BACKWARD_COMPAT_WARNED.discard(key)
        try:
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter("always")
                connection.arrow_number_to_decimal_setter = True
                connection.arrow_number_to_decimal_setter = False  # second set: deduped
        finally:
            if was_warned:
                _BACKWARD_COMPAT_WARNED.add(key)

        bc_warnings = [
            w
            for w in caught
            if issubclass(w.category, DeprecationWarning) and "arrow_number_to_decimal_setter" in str(w.message)
        ]
        assert len(bc_warnings) == 1, [str(w.message) for w in caught]

    def test_arrow_number_to_decimal_setter_disables(self, connection):
        connection.arrow_number_to_decimal = True
        connection.arrow_number_to_decimal = False
        assert connection.arrow_number_to_decimal is False

    def test_arrow_number_to_decimal_setter_coerces_to_bool(self, connection):
        connection.arrow_number_to_decimal = 1
        assert connection.arrow_number_to_decimal is True

        connection.arrow_number_to_decimal = 0
        assert connection.arrow_number_to_decimal is False

    def test_arrow_number_to_decimal_true_from_kwargs(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", arrow_number_to_decimal=True)
        assert conn.arrow_number_to_decimal is True

    def test_arrow_number_to_decimal_false_from_kwargs(self, mock_db_api):
        from snowflake.connector.connection import Connection

        conn = Connection(user="u", account="a", arrow_number_to_decimal=False)
        assert conn.arrow_number_to_decimal is False

    def test_arrow_number_to_decimal_not_leaked_to_rust_core(self, mock_db_api):
        from snowflake.connector.connection import Connection

        Connection(user="u", account="a", arrow_number_to_decimal=True)

        request = mock_db_api.connection_set_options.call_args_list[0][0][0]
        assert "arrow_number_to_decimal" not in request.options


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


class TestIsValid:
    """Unit tests for Connection.is_valid()."""

    def test_returns_true_when_heartbeat_succeeds(self, connection, mock_db_api):
        mock_db_api.connection_heartbeat.return_value = MagicMock(valid=True)
        assert connection.is_valid() is True

    def test_returns_false_when_heartbeat_reports_invalid(self, connection, mock_db_api):
        mock_db_api.connection_heartbeat.return_value = MagicMock(valid=False)
        assert connection.is_valid() is False

    def test_returns_false_when_closed(self, connection, mock_db_api):
        # First is_closed() returns False (close proceeds), then True (post-close)
        mock_db_api.connection_is_closed.side_effect = [
            ConnectionIsClosedResponse(is_closed=False),
            ConnectionIsClosedResponse(is_closed=True),
        ]
        connection.close()
        assert connection.is_valid() is False
        mock_db_api.connection_heartbeat.assert_not_called()

    def test_returns_false_on_exception(self, connection, mock_db_api):
        mock_db_api.connection_heartbeat.side_effect = RuntimeError("transport error")
        assert connection.is_valid() is False


class TestExpired:
    """Unit tests for the ``Connection.expired`` property.

    The property mirrors ``SnowflakeConnection.expired`` in the legacy
    Python connector — it is ``False`` for a fresh connection and ``True``
    once the driver has detected master-token expiry (GS code 390114 or
    time-based expiry check during a refresh attempt).
    """

    def test_returns_false_for_fresh_connection(self, connection, mock_db_api):
        """A brand-new connection must report expired=False."""
        mock_db_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=False)
        assert connection.expired is False
        mock_db_api.connection_is_expired.assert_called_once()

    def test_returns_true_when_core_reports_expired(self, connection, mock_db_api):
        """expired=True is forwarded from sf_core."""
        mock_db_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=True)
        assert connection.expired is True

    def test_returns_true_on_exception(self, connection, mock_db_api):
        """If the RPC throws (e.g. handle already released) expired fails closed
        and returns True rather than propagating — the connection may be unusable,
        so pools evict it (matches the async is_expired() coroutine)."""
        mock_db_api.connection_is_expired.side_effect = RuntimeError("handle gone")
        assert connection.expired is True

    def test_closing_does_not_set_expired(self, connection, mock_db_api):
        """Closing a connection must not affect the expired flag — they are
        orthogonal states."""
        mock_db_api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=False)
        mock_db_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=False)
        connection.close()
        assert connection.expired is False

    def test_expired_returns_bool(self, connection, mock_db_api):
        """The property must return a plain Python bool, not a protobuf bool."""
        mock_db_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=True)
        result = connection.expired
        assert type(result) is bool


class TestClientPrefetchThreadsProperty:
    """Unit tests for Connection.client_prefetch_threads getter and setter."""

    def test_should_return_default_value_of_4(self, connection):
        assert connection.client_prefetch_threads == 4

    def test_should_return_configured_value(self, mock_db_api):
        conn = Connection(user="test_user", account="test_account", client_prefetch_threads=8)
        conn.auto_cleanup = False
        assert conn.client_prefetch_threads == 8

    def test_should_update_value_via_setter(self, connection):
        """Setting the property should execute ALTER SESSION so it actually takes
        effect on subsequent fetches, not just update local state."""
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.client_prefetch_threads = 6

        mock_cursor.execute.assert_called_once_with("ALTER SESSION SET CLIENT_PREFETCH_THREADS = 6")
        assert connection.client_prefetch_threads == 6

    def test_should_roundtrip_set_then_get(self, connection):
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        for value in (1, 3, 8, 10):
            connection.client_prefetch_threads = value
            assert connection.client_prefetch_threads == value

    def test_setter_closes_cursor(self, connection):
        """The cursor opened to run ALTER SESSION should always be closed."""
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.client_prefetch_threads = 6

        mock_cursor.close.assert_called_once()

    @pytest.mark.parametrize(
        ("given", "expected"),
        [
            (-5, 1),
            (0, 1),
            (1, 1),
            (10, 10),
            (11, 10),
            (999, 10),
        ],
    )
    def test_setter_clamps_to_legacy_bounds(self, connection, given, expected):
        """Values outside [1, 10] should be clamped, matching the legacy connector's
        `_validate_client_prefetch_threads`."""
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.client_prefetch_threads = given

        assert connection.client_prefetch_threads == expected
        mock_cursor.execute.assert_called_once_with(f"ALTER SESSION SET CLIENT_PREFETCH_THREADS = {expected}")

    def test_kwarg_clamps_to_legacy_bounds(self, mock_db_api):
        """The constructor kwarg should be clamped the same way as the setter."""
        conn = Connection(user="test_user", account="test_account", client_prefetch_threads=999)
        conn.auto_cleanup = False
        assert conn.client_prefetch_threads == 10


class TestSetClientPrefetchThreads:
    """Unit tests for Connection.set_client_prefetch_threads / get_client_prefetch_threads."""

    def test_set_executes_alter_session(self, connection):
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.set_client_prefetch_threads(6)

        mock_cursor.execute.assert_called_once_with("ALTER SESSION SET CLIENT_PREFETCH_THREADS = 6")
        mock_cursor.close.assert_called_once()

    def test_get_reflects_last_set_value(self, connection):
        mock_cursor = MagicMock()
        connection.cursor = MagicMock(return_value=mock_cursor)

        connection.set_client_prefetch_threads(7)

        assert connection.get_client_prefetch_threads() == 7
