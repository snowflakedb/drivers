"""Unit tests for AsyncTelemetryClient's log-batch methods.

The coroutines are driven via ``asyncio.run`` with the async CoreDriver facade
mocked (``AsyncMock``) — no pytest-asyncio dependency needed. Mirrors the sync
``test_log_batch_telemetry`` state-machine coverage. The async wire path itself
is identical to sync (same RPCs and Rust core) and is proven by the sync
wiremock e2e plus the sf_core integration tests.
"""

from __future__ import annotations

import asyncio

from unittest.mock import AsyncMock, patch

import pytest

from snowflake.connector._common.telemetry import AsyncTelemetryClient
from snowflake.connector._internal.api_client.client_api import async_core_driver
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConnectionHandle
from snowflake.connector.errors import InterfaceError
from snowflake.connector.telemetry import TelemetryData


_CONN_HANDLE = ConnectionHandle(id=1)


def _client() -> AsyncTelemetryClient:
    return AsyncTelemetryClient(conn_handle=_CONN_HANDLE)


class TestAsyncAddLogToBatch:
    def test_forwards_entry_to_core(self):
        client = _client()
        telemetry_data = TelemetryData(message={"type": "ct", "value": 42}, timestamp=1700000000123)
        with patch.object(async_core_driver, "telemetry_send_log", new_callable=AsyncMock) as rpc:
            asyncio.run(client.add_log_to_batch(telemetry_data))
        rpc.assert_awaited_once_with(
            conn_handle=_CONN_HANDLE,
            message_json='{"type": "ct", "value": 42}',
            timestamp_ms=1700000000123,
        )

    def test_raises_when_closed(self):
        client = _client()
        client._closed = True
        with patch.object(async_core_driver, "telemetry_send_log", new_callable=AsyncMock) as rpc:
            with pytest.raises(InterfaceError):
                asyncio.run(client.add_log_to_batch(TelemetryData(message={"type": "ct"}, timestamp=1)))
            rpc.assert_not_awaited()


class TestAsyncTryAddLogToBatch:
    def test_swallows_closed_client_error(self):
        client = _client()
        client._closed = True
        with patch.object(async_core_driver, "telemetry_send_log", new_callable=AsyncMock) as rpc:
            asyncio.run(client.try_add_log_to_batch(TelemetryData(message={"type": "ct"}, timestamp=1)))
            rpc.assert_not_awaited()

    def test_swallows_non_serializable_message(self):
        client = _client()
        with patch.object(async_core_driver, "telemetry_send_log", new_callable=AsyncMock) as rpc:
            # object() is not JSON-serializable: json.dumps raises TypeError, which
            # the try_ wrapper swallows before the RPC is ever invoked.
            asyncio.run(client.try_add_log_to_batch(TelemetryData(message={"bad": object()}, timestamp=1)))
            rpc.assert_not_awaited()

    def test_forwards_on_happy_path(self):
        client = _client()
        with patch.object(async_core_driver, "telemetry_send_log", new_callable=AsyncMock) as rpc:
            asyncio.run(client.try_add_log_to_batch(TelemetryData(message={"type": "ok"}, timestamp=2)))
        rpc.assert_awaited_once()
