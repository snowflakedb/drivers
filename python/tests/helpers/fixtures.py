"""Shared pytest fixtures for Core mock and introspection."""

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest.mock import AsyncMock, MagicMock

import pytest


if TYPE_CHECKING:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ExecuteQueryResponse,
    )


def _make_execute_response(query_id: str = "fake-qid") -> ExecuteQueryResponse:
    """Return an ExecuteQueryResponse with a single-statement ResultSetResponse."""
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ExecuteQueryResponse,
        ResultSetDescriptor,
        ResultSetHandle,
        ResultSetResponse,
    )

    return ExecuteQueryResponse(
        single=ResultSetResponse(
            result_set_handle=ResultSetHandle(id=1),
            result_descriptor=ResultSetDescriptor(query_id=query_id),
        )
    )


@pytest.fixture
def mock_db_api():
    """MagicMock db_api patched into core_driver for Connection.__init__ tests.

    Sets ``core_driver.client`` to the mock so that all RPC calls flow
    through the mock. Restores the previous client on teardown.
    """
    from snowflake.connector._internal.api_client.client_api import core_driver
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ConnectionHandle,
        ConnectionIsClosedResponse,
        ConnectionIsExpiredResponse,
        ConnectionSetOptionsResponse,
        DatabaseHandle,
    )

    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    db_api.connection_get_parameter.return_value = MagicMock(value="")
    db_api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=False)
    db_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=False)
    db_api.connection_set_options.return_value = ConnectionSetOptionsResponse(warnings=[])

    old_client = core_driver._client
    core_driver.client = db_api
    yield db_api
    core_driver.client = old_client


@pytest.fixture
def mock_async_db_api():
    """MagicMock async db_api patched into async_core_driver for async Connection tests.

    Sets ``async_core_driver.client`` to the mock so that all RPC calls flow
    through the mock. Restores the previous client on teardown.
    """
    from snowflake.connector._internal.api_client.client_api import async_core_driver
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ConnectionHandle,
        ConnectionIsClosedResponse,
        ConnectionIsExpiredResponse,
        DatabaseHandle,
        ResultSetDescriptor,
        StatementHandle,
        TelemetrySendResponse,
    )

    db_api = MagicMock()
    db_api.database_new = AsyncMock(return_value=MagicMock(db_handle=DatabaseHandle(id=1)))
    db_api.database_init = AsyncMock()
    db_api.connection_new = AsyncMock(return_value=MagicMock(conn_handle=ConnectionHandle(id=42)))
    db_api.connection_set_options = AsyncMock(return_value=MagicMock(warnings=[]))
    db_api.connection_set_session_parameters = AsyncMock()
    db_api.connection_init = AsyncMock()
    db_api.connection_get_parameter = AsyncMock(return_value=MagicMock(value=""))

    def _connection_is_closed(request):
        return ConnectionIsClosedResponse(is_closed=request.conn_handle.id == 0)

    db_api.connection_is_closed = AsyncMock(side_effect=_connection_is_closed)
    db_api.connection_is_expired = AsyncMock(return_value=ConnectionIsExpiredResponse(is_expired=False))
    db_api.connection_close = AsyncMock()
    db_api.connection_release = AsyncMock()
    db_api.database_release = AsyncMock()
    db_api.connection_get_all_parameters = AsyncMock(return_value=MagicMock(parameters={}))
    db_api.connection_get_info = AsyncMock(return_value=MagicMock(ListFields=lambda: []))
    db_api.statement_new = AsyncMock(return_value=MagicMock(stmt_handle=StatementHandle(id=1)))
    db_api.statement_set_sql_query = AsyncMock()
    db_api.statement_execute_query = AsyncMock(return_value=_make_execute_response())
    db_api.statement_release = AsyncMock()
    db_api.connection_get_result_set = AsyncMock(
        return_value=MagicMock(result_descriptor=ResultSetDescriptor(query_id="fake-qid")),
    )
    db_api.telemetry_send_api_usage = AsyncMock(return_value=TelemetrySendResponse())

    old_client = async_core_driver._client
    async_core_driver.client = db_api
    yield db_api
    async_core_driver.client = old_client


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
