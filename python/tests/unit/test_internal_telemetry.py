"""Unit tests for Connection telemetry integration."""

import platform

from unittest.mock import AsyncMock, MagicMock

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionGetInfoResponse,
    ConnectionHandle,
    ConnectionInitResponse,
    ConnectionIsClosedResponse,
    ConnectionNewResponse,
    ConnectionSetOptionsResponse,
    ConnectionSetSessionParametersResponse,
    DatabaseHandle,
    DatabaseInitResponse,
    DatabaseNewResponse,
)
from snowflake.connector.version import __version__
from tests.compatibility import is_new_driver


pytestmark = pytest.mark.skipif(not is_new_driver(), reason="Requires universal driver")


class TestConnectionInitIdentity:
    """Tests that wrapper identity fields are passed in connection_init."""

    @pytest.fixture
    def full_mock_db_api(self):
        from snowflake.connector._internal.api_client.client_api import async_core_driver, core_driver

        db_api = MagicMock()
        db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
        db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
        db_api.connection_get_parameter.return_value = MagicMock(value="")

        old_client = core_driver._client
        core_driver.client = db_api

        async_api = AsyncMock()
        async_api.database_new.return_value = DatabaseNewResponse(db_handle=DatabaseHandle(id=1))
        async_api.database_init.return_value = DatabaseInitResponse()
        async_api.connection_new.return_value = ConnectionNewResponse(conn_handle=ConnectionHandle(id=42))
        async_api.connection_init.return_value = ConnectionInitResponse()
        async_api.connection_set_options.return_value = ConnectionSetOptionsResponse(warnings=[])
        async_api.connection_set_session_parameters.return_value = ConnectionSetSessionParametersResponse()
        async_api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=False)
        async_api.connection_get_parameter.return_value = MagicMock(value="")
        async_api.connection_get_all_parameters.return_value = MagicMock(parameters={})
        async_api.connection_get_info.return_value = ConnectionGetInfoResponse()
        old_async_client = async_core_driver._client
        async_core_driver.client = async_api

        yield async_api

        core_driver.client = old_client
        async_core_driver.client = old_async_client

    def test_connection_init_includes_identity_fields(self, full_mock_db_api):
        from snowflake.connector.connection import Connection

        Connection(user="test_user", account="test_account")

        full_mock_db_api.connection_init.assert_called_once()
        req = full_mock_db_api.connection_init.call_args[0][0]
        identity = req.wrapper_identity
        from snowflake.connector.aio.connection import _APPLICATION_NAME

        assert identity.driver_name == _APPLICATION_NAME
        assert identity.driver_version == __version__
        assert identity.language_runtime == platform.python_implementation()
        assert identity.language_version == platform.python_version()
        assert identity.language_compiler == platform.python_compiler()
