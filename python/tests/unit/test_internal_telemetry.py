"""Unit tests for _internal.telemetry module and Connection telemetry integration."""

import platform

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
)
from snowflake.connector.version import __version__
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


@pytest.fixture
def mock_db_api():
    return MagicMock()


@pytest.fixture
def conn_handle():
    return ConnectionHandle(id=42)


@pytest.fixture
def telemetry_client(mock_db_api, conn_handle):
    from snowflake.connector._internal.telemetry import TelemetryClient

    return TelemetryClient(mock_db_api, conn_handle)


class TestTelemetryClientSendApiUsage:
    """Tests for TelemetryClient.send_api_usage."""

    def test_calls_rpc_with_correct_args(self, telemetry_client, mock_db_api, conn_handle):
        telemetry_client.send_api_usage("cursor.execute")

        mock_db_api.telemetry_send_api_usage.assert_called_once()
        req = mock_db_api.telemetry_send_api_usage.call_args[0][0]
        assert req.conn_handle == conn_handle
        assert req.api_method == "cursor.execute"

    def test_swallows_exceptions(self, telemetry_client, mock_db_api):
        mock_db_api.telemetry_send_api_usage.side_effect = RuntimeError("rpc failed")

        # Should not raise
        telemetry_client.send_api_usage("cursor.execute")


class TestTelemetryClientSendWrapperError:
    """Tests for TelemetryClient.send_wrapper_error."""

    def test_calls_rpc_with_correct_args(self, telemetry_client, mock_db_api, conn_handle):
        telemetry_client.send_wrapper_error("ProgrammingError", "cursor.execute")

        mock_db_api.telemetry_send_wrapper_error.assert_called_once()
        req = mock_db_api.telemetry_send_wrapper_error.call_args[0][0]
        assert req.conn_handle == conn_handle
        assert req.exception_type == "ProgrammingError"
        assert req.error_source == "cursor.execute"

    def test_swallows_exceptions(self, telemetry_client, mock_db_api):
        mock_db_api.telemetry_send_wrapper_error.side_effect = RuntimeError("rpc failed")

        # Should not raise
        telemetry_client.send_wrapper_error("Error", "source")


class TestConnectionInitIdentity:
    """Tests that wrapper identity fields are passed in connection_init."""

    @pytest.fixture
    def full_mock_db_api(self):
        db_api = MagicMock()
        db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
        db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
        db_api.connection_get_parameter.return_value = MagicMock(value="")
        return db_api

    def test_connection_init_includes_identity_fields(self, full_mock_db_api):
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.database_driver_client", return_value=full_mock_db_api):
            Connection(user="test_user", account="test_account")

        full_mock_db_api.connection_init.assert_called_once()
        req = full_mock_db_api.connection_init.call_args[0][0]
        assert req.driver_name == "snowflake-connector-python"
        assert req.driver_version == __version__
        assert req.language_runtime == platform.python_implementation()
        assert req.language_version == platform.python_version()
        assert req.language_compiler == platform.python_compiler()
