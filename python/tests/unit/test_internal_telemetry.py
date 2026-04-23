"""Unit tests for Connection telemetry integration."""

import platform

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
)
from snowflake.connector.version import __version__
from tests.compatibility import is_new_driver


pytestmark = pytest.mark.skipif(not is_new_driver(), reason="Requires universal driver")


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
        identity = req.wrapper_identity
        from snowflake.connector.connection import CLIENT_NAME

        assert identity.driver_name == CLIENT_NAME
        assert identity.driver_version == __version__
        assert identity.language_runtime == platform.python_implementation()
        assert identity.language_version == platform.python_version()
        assert identity.language_compiler == platform.python_compiler()
