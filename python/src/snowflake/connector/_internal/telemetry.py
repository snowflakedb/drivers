"""In-band telemetry client that sends events to sf_core via protobuf RPC.

All methods are fire-and-forget: failures are logged at DEBUG level and never
propagate to the caller.
"""

from __future__ import annotations

import json
import logging

from typing import TYPE_CHECKING

from ..errors import InterfaceError
from .api_client.client_api import async_core_driver, core_driver


if TYPE_CHECKING:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
        ConnectionHandle,
    )
    from snowflake.connector.telemetry import TelemetryData

logger = logging.getLogger(__name__)


class TelemetryClient:
    """Sends telemetry events to sf_core via protobuf RPC.

    Wrapper identity is passed as part of ``connection_init``. This client only
    sends runtime events — sf_core attaches the stored identity automatically.
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle
        # Log-batch lifecycle flags mirroring the legacy TelemetryClient:
        # _enabled is a client kill-switch; _closed rejects further use after close().
        self._enabled = True
        self._closed = False

    def send_api_usage(self, api_method: str, passed_arguments: list[str] | None = None) -> None:
        """Record an API method call for telemetry.

        ``passed_arguments`` lists the names of the arguments the caller
        explicitly supplied (names only, no values, defaults omitted).
        """
        try:
            core_driver.telemetry_send_api_usage(
                conn_handle=self._conn_handle,
                api_method=api_method,
                passed_arguments=passed_arguments or [],
            )
        except Exception:
            logger.debug("Failed to send api_usage telemetry", exc_info=True)

    def send_wrapper_error(self, exception_type: str, error_source: str) -> None:
        """Record a wrapper error for telemetry."""
        try:
            core_driver.telemetry_send_wrapper_error(
                conn_handle=self._conn_handle,
                exception_type=exception_type,
                error_source=error_source,
            )
        except Exception:
            logger.debug("Failed to send wrapper_error telemetry", exc_info=True)

    def add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Buffer one caller-produced telemetry entry (e.g. Snowpark's) on the
        connection's in-band log batch, forwarding it to sf_core over RPC.

        Wire batching, the flush threshold, and ``/telemetry/send`` egress all
        live in the Rust core; this method only gates and forwards one entry.
        The core keys the batch by session, so ``timestamp`` is the caller's
        event time (epoch ms) and ``message`` is preserved verbatim.
        """
        # Closed before disabled: adding after close() is a programming error and
        # must surface (try_add_log_to_batch swallows it for the hot path), whereas
        # a kill-switched client is a silent no-op.
        if self._closed:
            raise InterfaceError("Attempted to add log when TelemetryClient is closed")
        if not self._enabled:
            return
        # Strict on purpose: json.dumps / RPC errors propagate so the caller
        # (try_add_log_to_batch) decides whether to swallow them.
        core_driver.telemetry_add_log_to_batch(
            conn_handle=self._conn_handle,
            message_json=json.dumps(telemetry_data.message),
            timestamp_ms=int(telemetry_data.timestamp),
        )

    def try_add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Exception-swallowing wrapper over :meth:`add_log_to_batch` — the hot
        path callers use. A closed client (which raises) or a non-serializable
        ``message`` must never surface into caller code.
        """
        try:
            self.add_log_to_batch(telemetry_data)
        except Exception:
            logger.debug("Failed to add log to telemetry", exc_info=True)

    def send_log_batch(self) -> None:
        """Flush the connection's buffered log-telemetry batch to Snowflake.

        A no-op when telemetry is disabled. A send failure quiesces telemetry for
        this client (best-effort: telemetry must never break the caller).
        """
        if not self._enabled:
            return
        try:
            core_driver.telemetry_send_log_batch(conn_handle=self._conn_handle)
        except Exception:
            logger.debug("Failed to send telemetry log batch", exc_info=True)
            self._enabled = False

    # Backward-compatibility alias: snowflake-connector-python named this _log_batch.
    _log_batch = send_log_batch

    def close(self) -> None:
        """Flush any buffered entries, then reject further use. Idempotent."""
        if self._closed:
            return
        self.send_log_batch()
        self._closed = True


class AsyncTelemetryClient:
    """Async counterpart of :class:`TelemetryClient` for :class:`~snowflake.connector.aio.Connection`.

    Uses :data:`~snowflake.connector._internal.api_client.client_api.async_core_driver`
    so telemetry RPCs do not block the event loop.
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle
        self._enabled = True
        self._closed = False

    async def send_api_usage(self, api_method: str, passed_arguments: list[str] | None = None) -> None:
        """Record an API method call for telemetry.

        ``passed_arguments`` lists the names of the arguments the caller
        explicitly supplied (names only, no values, defaults omitted).
        """
        try:
            await async_core_driver.telemetry_send_api_usage(
                conn_handle=self._conn_handle,
                api_method=api_method,
                passed_arguments=passed_arguments or [],
            )
        except Exception:
            logger.debug("Failed to send api_usage telemetry", exc_info=True)

    async def send_wrapper_error(self, exception_type: str, error_source: str) -> None:
        """Record a wrapper error for telemetry."""
        try:
            await async_core_driver.telemetry_send_wrapper_error(
                conn_handle=self._conn_handle,
                exception_type=exception_type,
                error_source=error_source,
            )
        except Exception:
            logger.debug("Failed to send wrapper_error telemetry", exc_info=True)

    async def add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Buffer one caller-produced telemetry entry, forwarding it to sf_core.

        Async counterpart of :meth:`TelemetryClient.add_log_to_batch`: raises when
        closed, silent no-op when disabled, otherwise forwards one entry (strict on
        purpose — the try_ variant owns error swallowing).
        """
        if self._closed:
            raise InterfaceError("Attempted to add log when TelemetryClient is closed")
        if not self._enabled:
            return
        await async_core_driver.telemetry_add_log_to_batch(
            conn_handle=self._conn_handle,
            message_json=json.dumps(telemetry_data.message),
            timestamp_ms=int(telemetry_data.timestamp),
        )

    async def try_add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Exception-swallowing wrapper over :meth:`add_log_to_batch` (the hot path)."""
        try:
            await self.add_log_to_batch(telemetry_data)
        except Exception:
            logger.debug("Failed to add log to telemetry", exc_info=True)

    async def send_log_batch(self) -> None:
        """Flush the connection's buffered log-telemetry batch to Snowflake.

        A no-op when telemetry is disabled. A send failure quiesces telemetry for
        this client (best-effort: telemetry must never break the caller).
        """
        if not self._enabled:
            return
        try:
            await async_core_driver.telemetry_send_log_batch(conn_handle=self._conn_handle)
        except Exception:
            logger.debug("Failed to send telemetry log batch", exc_info=True)
            self._enabled = False

    # Backward-compatibility alias: snowflake-connector-python named this _log_batch.
    _log_batch = send_log_batch

    async def close(self) -> None:
        """Flush any buffered entries, then reject further use. Idempotent."""
        if self._closed:
            return
        await self.send_log_batch()
        self._closed = True
