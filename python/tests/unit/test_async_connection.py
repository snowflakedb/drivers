"""Unit tests for AsyncConnection lifecycle."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionCloseResponse,
    ConnectionGetInfoResponse,
    ConnectionGetQueryStatusResponse,
    ConnectionHandle,
    ConnectionInitResponse,
    ConnectionIsClosedResponse,
    ConnectionNewResponse,
    ConnectionReleaseResponse,
    ConnectionSetOptionsResponse,
    ConnectionSetSessionParametersResponse,
    DatabaseHandle,
    DatabaseInitResponse,
    DatabaseNewResponse,
    DatabaseReleaseResponse,
    ExecuteQueryResponse,
    ResultSetDescriptor,
    ResultSetHandle,
    ResultSetResponse,
    StatementHandle,
)
from snowflake.connector.constants import QueryStatus
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


def _make_async_mock():
    """Build an AsyncMock configured for AsyncConnection lifecycle."""
    api = AsyncMock()
    api.database_new.return_value = DatabaseNewResponse(db_handle=DatabaseHandle(id=10))
    api.database_init.return_value = DatabaseInitResponse()
    api.connection_new.return_value = ConnectionNewResponse(conn_handle=ConnectionHandle(id=99))
    api.connection_init.return_value = ConnectionInitResponse()
    api.connection_set_options.return_value = ConnectionSetOptionsResponse(warnings=[])
    api.connection_set_session_parameters.return_value = ConnectionSetSessionParametersResponse()
    api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=False)
    api.connection_get_parameter.return_value = MagicMock(value="")
    api.connection_get_all_parameters.return_value = MagicMock(parameters={})
    api.connection_get_info.return_value = ConnectionGetInfoResponse()
    api.connection_close.return_value = ConnectionCloseResponse()
    api.connection_release.return_value = ConnectionReleaseResponse()
    api.database_release.return_value = DatabaseReleaseResponse()
    api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
    api.statement_set_sql_query.return_value = MagicMock()
    api.statement_execute_query.return_value = ExecuteQueryResponse(
        single=ResultSetResponse(
            result_set_handle=ResultSetHandle(id=1),
            result_descriptor=ResultSetDescriptor(query_id="fake-qid"),
        ),
    )
    api.statement_release.return_value = MagicMock()
    api.result_set_release.return_value = MagicMock()
    return api


async def _create_conn(api):
    from snowflake.connector._internal.api_client.client_api import async_core_driver
    from snowflake.connector.aio.connection import AsyncConnection

    old = async_core_driver._client
    async_core_driver.client = api
    try:
        conn = AsyncConnection(user="test_user", account="test_account")
        await conn.connect()
    except Exception:
        async_core_driver.client = old
        raise
    return conn, old


async def _cleanup(api_old):
    from snowflake.connector._internal.api_client.client_api import async_core_driver

    async_core_driver.client = api_old


class TestAsyncConnectionCreate:
    @pytest.mark.asyncio
    async def test_connect_calls_connection_init(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_init.assert_called_once()
            req = api.connection_init.call_args[0][0]
            assert req.conn_handle == ConnectionHandle(id=99)
            assert req.db_handle == DatabaseHandle(id=10)
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_connect_sets_handles(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            assert conn.conn_handle == ConnectionHandle(id=99)
            assert conn.db_handle == DatabaseHandle(id=10)
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_constructor_stores_config(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            assert conn.config.user == "test_user"
            assert conn.config.account == "test_account"
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_constructor_is_sync(self):
        """AsyncConnection() does not require await — only connect() does."""
        from snowflake.connector.aio.connection import AsyncConnection

        conn = AsyncConnection(user="u", account="a")
        assert conn.conn_handle is None
        assert not conn._connected


class TestAsyncConnectionClose:
    @pytest.mark.asyncio
    async def test_close_sends_close_and_release(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            await conn.close()
            api.connection_close.assert_called_once()
            api.connection_release.assert_called_once()
            api.database_release.assert_called_once()
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_close_is_idempotent(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_is_closed.side_effect = [
                ConnectionIsClosedResponse(is_closed=False),
                ConnectionIsClosedResponse(is_closed=True),
            ]
            await conn.close()
            await conn.close()
            api.connection_close.assert_called_once()
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_close_freezes_proxies(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            await conn.close()
            api.connection_get_all_parameters.assert_called_once()
            api.connection_get_info.assert_called_once()
        finally:
            await _cleanup(old)


class TestAsyncConnectionContextManager:
    @pytest.mark.asyncio
    async def test_async_with_closes(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            async with conn:
                assert not await conn.is_closed()
            api.connection_close.assert_called_once()
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_aenter_calls_connect_if_not_connected(self):
        """async with on an unconnnected instance should call connect()."""
        api = _make_async_mock()
        from snowflake.connector._internal.api_client.client_api import async_core_driver
        from snowflake.connector.aio.connection import AsyncConnection

        old = async_core_driver._client
        async_core_driver.client = api
        try:
            conn = AsyncConnection(user="u", account="a")
            assert not conn._connected
            async with conn:
                assert conn._connected
                assert conn.conn_handle == ConnectionHandle(id=99)
        finally:
            async_core_driver.client = old


class TestAsyncConnectionIsValid:
    @pytest.mark.asyncio
    async def test_is_valid_true(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_heartbeat.return_value = MagicMock(valid=True)
            assert await conn.is_valid()
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_is_valid_false_on_bad_heartbeat(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_heartbeat.return_value = MagicMock(valid=False)
            assert not await conn.is_valid()
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_is_valid_false_when_closed(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=True)
            assert not await conn.is_valid()
        finally:
            await _cleanup(old)


class TestAsyncConnectionProperties:
    @pytest.mark.asyncio
    async def test_info_properties(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_get_info.return_value = ConnectionGetInfoResponse(
                role="ANALYST",
                database="TEST_DB",
                schema="PUBLIC",
                account="TEST_ACCT",
                warehouse="TEST_WH",
                user="test_user",
                host="test.snowflakecomputing.com",
                port=443,
            )
            assert conn.role == "ANALYST"
            assert conn.database == "TEST_DB"
            assert conn.schema == "PUBLIC"
            assert conn.warehouse == "TEST_WH"
            assert conn.user == "test_user"
            assert conn.host == "test.snowflakecomputing.com"
            assert conn.port == 443
        finally:
            await _cleanup(old)


class TestAsyncConnectionQueryStatus:
    @pytest.mark.asyncio
    async def test_get_query_status(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
                status_name="SUCCESS",
            )
            status = await conn.get_query_status("some-query-id")
            assert status == QueryStatus.SUCCESS
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_get_query_status_throw_if_error(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            api.connection_get_query_status.return_value = ConnectionGetQueryStatusResponse(
                status_name="FAILED_WITH_ERROR",
                error_message="syntax error",
                error_code=1003,
            )
            with pytest.raises(Exception, match="syntax error"):
                await conn.get_query_status_throw_if_error("bad-query-id")
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_static_helpers(self):
        from snowflake.connector.aio.connection import AsyncConnection

        assert AsyncConnection.is_still_running(QueryStatus.RUNNING)
        assert not AsyncConnection.is_still_running(QueryStatus.SUCCESS)
        assert AsyncConnection.is_an_error(QueryStatus.FAILED_WITH_ERROR)
        assert not AsyncConnection.is_an_error(QueryStatus.SUCCESS)


class TestAsyncConnectionCursor:
    @pytest.mark.asyncio
    async def test_cursor_returns_async_cursor(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            from snowflake.connector.aio.cursor import AsyncSnowflakeCursor

            cur = conn.cursor()
            assert isinstance(cur, AsyncSnowflakeCursor)
        finally:
            await _cleanup(old)

    @pytest.mark.asyncio
    async def test_cursor_execute_lifecycle(self):
        api = _make_async_mock()
        conn, old = await _create_conn(api)
        try:
            with patch("snowflake.connector.cursor._query_result.get_stream_ptr", return_value=0):
                cur = conn.cursor()
                await cur.execute("SELECT 1")
                api.statement_execute_query.assert_called_once()
                await cur.close()
        finally:
            await _cleanup(old)


class TestAsyncConnect:
    @pytest.mark.asyncio
    async def test_aio_connect(self):
        api = _make_async_mock()
        from snowflake.connector._internal.api_client.client_api import async_core_driver

        old = async_core_driver._client
        async_core_driver.client = api
        try:
            from snowflake.connector.aio import connect

            conn = await connect(user="u", account="a")
            assert conn.conn_handle == ConnectionHandle(id=99)
            await conn.close()
        finally:
            async_core_driver.client = old
