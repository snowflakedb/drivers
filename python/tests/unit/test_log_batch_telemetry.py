"""Unit tests for TelemetryClient.send_log_batch and the _log_batch compat alias.

The Universal Driver sends telemetry via RPC immediately (no internal batch
buffer).  send_log_batch() is a no-op retained for backward compatibility with
snowflake-connector-python callers.  _log_batch is an alias for send_log_batch
so that Snowpark test helpers that access ``_telemetry_client._log_batch``
continue to work without modification.
"""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from snowflake.connector._internal.api_client.client_api import core_driver
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConnectionHandle
from snowflake.connector._internal.telemetry import TelemetryClient


_CONN_HANDLE = ConnectionHandle(id=1)


class TestSendLogBatch:
    def test_send_log_batch_exists(self):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        assert callable(client.send_log_batch)

    def test_send_log_batch_returns_none(self):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        result = client.send_log_batch()
        assert result is None

    def test_send_log_batch_does_not_raise(self):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        client.send_log_batch()  # must not raise


class TestLogBatchAlias:
    def test_log_batch_exists(self):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        assert callable(client._log_batch)

    def test_log_batch_is_alias_for_send_log_batch(self):
        assert TelemetryClient._log_batch is TelemetryClient.send_log_batch

    def test_log_batch_returns_none(self):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        result = client._log_batch()
        assert result is None

    def test_log_batch_does_not_raise(self):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        client._log_batch()  # must not raise


class TestSendLogBatchDoesNotAffectOtherTelemetry:
    """send_log_batch must not disrupt normal RPC-based telemetry."""

    @pytest.fixture
    def mock_core_driver(self):
        mock = MagicMock()
        old = core_driver._client
        core_driver.client = mock
        yield mock
        core_driver.client = old

    def test_send_api_usage_still_works_after_send_log_batch(self, mock_core_driver):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        client.send_log_batch()
        client.send_api_usage("Connection.cursor")
        mock_core_driver.telemetry_send_api_usage.assert_called_once()

    def test_log_batch_alias_does_not_disrupt_send_api_usage(self, mock_core_driver):
        client = TelemetryClient(conn_handle=_CONN_HANDLE)
        client._log_batch()
        client.send_api_usage("Connection.close")
        mock_core_driver.telemetry_send_api_usage.assert_called_once()
