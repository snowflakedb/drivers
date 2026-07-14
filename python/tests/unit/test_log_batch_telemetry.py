"""Unit tests for the in-band log-telemetry client (add / try / send / close).

These methods forward caller-produced telemetry to sf_core over RPC; the wire
batching and ``/telemetry/send`` egress live in the Rust core (exercised by the
sf_core integration tests). Here we test the Python client's gating state
machine and error posture in isolation, with the CoreDriver facade mocked.
"""

from __future__ import annotations

from unittest.mock import patch

import pytest

from snowflake.connector._internal.api_client.client_api import core_driver
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConnectionHandle
from snowflake.connector._internal.telemetry import TelemetryClient
from snowflake.connector.errors import InterfaceError
from snowflake.connector.telemetry import TelemetryData
from tests.compatibility import NEW_DRIVER_ONLY


_CONN_HANDLE = ConnectionHandle(id=1)


def _client() -> TelemetryClient:
    return TelemetryClient(conn_handle=_CONN_HANDLE)


class TestAddLogToBatch:
    def test_forwards_entry_to_core(self):
        client = _client()
        telemetry_data = TelemetryData(message={"type": "ct", "value": 42}, timestamp=1700000000123)
        with patch.object(core_driver, "telemetry_add_log_to_batch") as rpc:
            client.add_log_to_batch(telemetry_data)
        rpc.assert_called_once_with(
            conn_handle=_CONN_HANDLE,
            message_json='{"type": "ct", "value": 42}',
            timestamp_ms=1700000000123,
        )

    def test_raises_when_closed(self):
        client = _client()
        client.close()
        with patch.object(core_driver, "telemetry_add_log_to_batch") as rpc:
            with pytest.raises(InterfaceError):
                client.add_log_to_batch(TelemetryData(message={"type": "ct"}, timestamp=1))
            rpc.assert_not_called()

    def test_disabled_is_silent_noop(self):
        client = _client()
        client._enabled = False
        with patch.object(core_driver, "telemetry_add_log_to_batch") as rpc:
            client.add_log_to_batch(TelemetryData(message={"type": "ct"}, timestamp=1))
            rpc.assert_not_called()


class TestTryAddLogToBatch:
    def test_swallows_closed_client_error(self):
        client = _client()
        client.close()
        with patch.object(core_driver, "telemetry_add_log_to_batch") as rpc:
            # A closed client makes add_log_to_batch raise; the try_ variant must swallow.
            client.try_add_log_to_batch(TelemetryData(message={"type": "ct"}, timestamp=1))
            rpc.assert_not_called()

    def test_swallows_non_serializable_message(self):
        client = _client()
        with patch.object(core_driver, "telemetry_add_log_to_batch") as rpc:
            # object() is not JSON-serializable: json.dumps raises TypeError, which
            # the try_ wrapper swallows before the RPC is ever invoked.
            client.try_add_log_to_batch(TelemetryData(message={"bad": object()}, timestamp=1))
            rpc.assert_not_called()

    def test_forwards_on_happy_path(self):
        client = _client()
        with patch.object(core_driver, "telemetry_add_log_to_batch") as rpc:
            client.try_add_log_to_batch(TelemetryData(message={"type": "ok"}, timestamp=2))
        rpc.assert_called_once()


class TestSendLogBatch:
    def test_forwards_to_core(self):
        client = _client()
        with patch.object(core_driver, "telemetry_send_log_batch") as rpc:
            client.send_log_batch()
        rpc.assert_called_once_with(conn_handle=_CONN_HANDLE)

    def test_disabled_is_noop(self):
        client = _client()
        client._enabled = False
        with patch.object(core_driver, "telemetry_send_log_batch") as rpc:
            client.send_log_batch()
            rpc.assert_not_called()

    def test_disables_client_on_rpc_failure(self):
        client = _client()
        with patch.object(core_driver, "telemetry_send_log_batch", side_effect=RuntimeError("boom")):
            client.send_log_batch()  # must not raise
        assert client._enabled is False

    def test_bd42_http_failure_swallowed_in_core_client_stays_enabled(self):
        """BD#42: the Rust core swallows HTTP-layer failures and returns RPC success,
        so the Python client never sees an exception and _enabled is NOT cleared.

        Old driver (OLD_DRIVER_ONLY): an HTTP error or ``success: false`` response
        from ``/telemetry/send`` sets TelemetryClient._enabled = False immediately,
        quiescing all further log entries for the connection lifetime.
        """
        if not NEW_DRIVER_ONLY("BD#42"):
            return
        client = _client()
        # No side_effect = RPC returns success, simulating Rust having swallowed an HTTP error.
        with patch.object(core_driver, "telemetry_send_log_batch"):
            client.send_log_batch()
        assert client._enabled is True


class TestLogBatchAlias:
    def test_log_batch_is_alias_for_send_log_batch(self):
        assert TelemetryClient._log_batch is TelemetryClient.send_log_batch


class TestClose:
    def test_close_flushes_then_marks_closed(self):
        client = _client()
        with patch.object(core_driver, "telemetry_send_log_batch") as rpc:
            client.close()
        rpc.assert_called_once_with(conn_handle=_CONN_HANDLE)
        assert client._closed is True

    def test_close_is_idempotent(self):
        client = _client()
        with patch.object(core_driver, "telemetry_send_log_batch") as rpc:
            client.close()
            client.close()  # second close must not flush again
        rpc.assert_called_once()
