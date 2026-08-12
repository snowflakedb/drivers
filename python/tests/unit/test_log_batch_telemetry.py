"""Unit tests for the in-band log-telemetry client (add / try / send_batch).

These methods forward caller-produced telemetry to sf_core over RPC; the wire
batching and ``/telemetry/send`` egress live in the Rust core (exercised by the
sf_core integration tests). Here we test the Python client's gating state
machine and error posture in isolation, with the CoreDriver facade mocked.
"""

from __future__ import annotations

from unittest.mock import ANY, patch

import pytest

from snowflake.connector._internal.api_client.client_api import core_driver
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConnectionHandle
from snowflake.connector._internal.telemetry import TelemetryClient
from snowflake.connector.errors import InterfaceError
from snowflake.connector.telemetry import TelemetryData


_CONN_HANDLE = ConnectionHandle(id=1)


def _client() -> TelemetryClient:
    return TelemetryClient(conn_handle=_CONN_HANDLE)


class TestAddLogToBatch:
    def test_forwards_entry_to_core(self):
        client = _client()
        telemetry_data = TelemetryData(message={"type": "ct", "value": 42}, timestamp=1700000000123)
        with patch.object(core_driver, "telemetry_send_log") as rpc:
            client.add_log_to_batch(telemetry_data)
        rpc.assert_called_once_with(
            conn_handle=_CONN_HANDLE,
            message_json='{"type": "ct", "value": 42}',
            timestamp_ms=1700000000123,
        )

    def test_raises_when_closed(self):
        client = _client()
        client._closed = True
        with patch.object(core_driver, "telemetry_send_log") as rpc:
            with pytest.raises(InterfaceError):
                client.add_log_to_batch(TelemetryData(message={"type": "ct"}, timestamp=1))
            rpc.assert_not_called()


class TestTryAddLogToBatch:
    def test_swallows_closed_client_error(self):
        client = _client()
        client._closed = True
        with patch.object(core_driver, "telemetry_send_log") as rpc:
            # A closed client makes add_log_to_batch raise; the try_ variant must swallow.
            client.try_add_log_to_batch(TelemetryData(message={"type": "ct"}, timestamp=1))
            rpc.assert_not_called()

    def test_swallows_non_serializable_message(self):
        client = _client()
        with patch.object(core_driver, "telemetry_send_log") as rpc:
            # object() is not JSON-serializable: json.dumps raises TypeError, which
            # the try_ wrapper swallows before the RPC is ever invoked.
            client.try_add_log_to_batch(TelemetryData(message={"bad": object()}, timestamp=1))
            rpc.assert_not_called()

    def test_forwards_on_happy_path(self):
        client = _client()
        with patch.object(core_driver, "telemetry_send_log") as rpc:
            client.try_add_log_to_batch(TelemetryData(message={"type": "ok"}, timestamp=2))
        rpc.assert_called_once_with(conn_handle=ANY, message_json=ANY, timestamp_ms=ANY)


class TestSendBatch:
    def test_is_noop(self):
        # Core owns flush; send_batch exists for snowflake-cli API compatibility only.
        client = _client()
        with patch.object(core_driver, "telemetry_send_log") as rpc:
            client.send_batch()
            rpc.assert_not_called()
