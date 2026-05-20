"""Shared pytest fixtures for Core mock and introspection."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock

import pytest


@pytest.fixture
def mock_db_api():
    """MagicMock db_api patched into core_driver and async_core_driver.

    Sets ``core_driver.client`` and ``async_core_driver.client`` to mocks
    so that all RPC calls (sync and async) flow through the mock. Restores
    the previous clients on teardown.
    """
    from snowflake.connector._internal.api_client.client_api import async_core_driver, core_driver
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ConnectionHandle,
        ConnectionIsClosedResponse,
        ConnectionSetOptionsResponse,
        DatabaseHandle,
        ExecuteQueryResponse,
        ResultSetDescriptor,
        ResultSetHandle,
        ResultSetResponse,
        StatementHandle,
    )

    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    db_api.connection_get_parameter.return_value = MagicMock(value="")
    db_api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=False)
    db_api.connection_set_options.return_value = ConnectionSetOptionsResponse(warnings=[])

    old_client = core_driver._client
    core_driver.client = db_api

    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ConnectionCloseResponse,
        ConnectionGetInfoResponse,
        ConnectionInitResponse,
        ConnectionNewResponse,
        ConnectionReleaseResponse,
        ConnectionSetSessionParametersResponse,
        DatabaseInitResponse,
        DatabaseNewResponse,
        DatabaseReleaseResponse,
    )

    async_api = AsyncMock()
    # Connection lifecycle mocks (used by AsyncConnection.create)
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
    async_api.connection_close.return_value = ConnectionCloseResponse()
    async_api.connection_release.return_value = ConnectionReleaseResponse()
    async_api.database_release.return_value = DatabaseReleaseResponse()
    # Statement lifecycle mocks (used by cursors)
    async_api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
    async_api.statement_set_sql_query.return_value = MagicMock()
    async_api.statement_execute_query.return_value = ExecuteQueryResponse(
        single=ResultSetResponse(
            result_set_handle=ResultSetHandle(id=1),
            result_descriptor=ResultSetDescriptor(query_id="fake-qid"),
        )
    )
    async_api.statement_release.return_value = MagicMock()
    async_api.result_set_release.return_value = MagicMock()
    old_async_client = async_core_driver._client
    async_core_driver.client = async_api

    yield async_api

    core_driver.client = old_client
    async_core_driver.client = old_async_client


@pytest.fixture
def core_proxy():
    """Wrap real Core client with MagicMock recording. Universal driver only.

    Lazy imports avoid module-level _internal dependency — safe for reference
    connector collection. Tests using this fixture must be marked @skip_reference.
    """
    from snowflake.connector._internal.api_client.client_api import core_driver
    from tests.helpers.core_introspection import CoreIntrospector

    real_client = core_driver.client
    spy = MagicMock(wraps=real_client)
    core_driver.client = spy
    yield CoreIntrospector(spy)
    core_driver.client = real_client
