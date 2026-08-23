"""In-band telemetry client that sends events to sf_core via protobuf RPC.

All methods are fire-and-forget: failures are logged at DEBUG level and never
propagate to the caller.
"""

from __future__ import annotations

import json

from enum import Enum, unique
from typing import TYPE_CHECKING, Any

from .._internal.api_client.client_api import async_core_driver, core_driver
from .._internal.backward_compatibility import install_backward_compatibility_getattr
from .._internal.decorators import backward_compatibility
from .._internal.logging import get_logger
from ..errors import InterfaceError


if TYPE_CHECKING:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
        ConnectionHandle,
    )

logger = get_logger(__name__)


@backward_compatibility
@unique
class TelemetryField(Enum):
    """Old-driver keys for telemetry message dicts."""

    KEY_SOURCE = "source"
    KEY_TYPE = "type"
    KEY_SFQID = "query_id"


@backward_compatibility
class TelemetryData:
    """Old-driver telemetry data holder, forwarded to :class:`TelemetryClient`."""

    TRUE = "true"
    FALSE = "false"

    def __init__(self, message: Any = None, timestamp: int = 0) -> None:
        self.message = message
        self.timestamp = timestamp

    @classmethod
    def from_telemetry_data_dict(
        cls,
        from_dict: dict[str, Any],
        timestamp: int,
        connection: Any = None,
        is_oob_telemetry: bool = False,
    ) -> TelemetryData:
        return cls(message=from_dict, timestamp=timestamp)

    def to_dict(self) -> dict[str, Any]:
        return {"message": self.message, "timestamp": str(self.timestamp)}


class TelemetryClient:
    """Sends telemetry events to sf_core via protobuf RPC.

    Wrapper identity is passed as part of ``connection_init``. This client only
    sends runtime events — sf_core attaches the stored identity automatically.
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle
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
        """Forward one caller-produced telemetry entry (e.g. Snowpark's) to sf_core.

        Core owns batching, flush threshold, and ``/telemetry/send`` egress.
        Raises :class:`~snowflake.connector.errors.InterfaceError` if the client
        has been closed; use :meth:`try_add_log_to_batch` for the fire-and-forget
        hot path.
        """
        if self._closed:
            raise InterfaceError(
                "Cannot add log to batch: TelemetryClient is closed. Obtain a fresh client from a new connection."
            )
        core_driver.telemetry_send_log(
            conn_handle=self._conn_handle,
            message_json=json.dumps(telemetry_data.message),
            timestamp_ms=int(telemetry_data.timestamp),
        )

    def try_add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Exception-swallowing wrapper over :meth:`add_log_to_batch` — the hot path."""
        try:
            self.add_log_to_batch(telemetry_data)
        except Exception:
            logger.debug("Failed to add log to telemetry", exc_info=True)

    def send_batch(self) -> None:
        """No-op: flush is owned by the Rust core (threshold + connection release).

        Only an explicit close() or normal process exit (atexit) reliably
        flushes buffered telemetry. Neither this driver nor the legacy
        connector installs a signal handler, so a killed process (SIGTERM or
        SIGKILL) loses in-buffer entries — atexit does not run on signals.
        See ``tests/integ/telemetry/test_telemetry_crash_flush.py``.

        Kept for snowflake-cli API compatibility (``_app/telemetry.py``).
        """

    @backward_compatibility
    def send_log_batch(self) -> None:
        """No-op: the new driver sends telemetry via RPC immediately; there is no batch to flush."""


class AsyncTelemetryClient:
    """Async counterpart of :class:`TelemetryClient` for :class:`~snowflake.connector.aio.Connection`.

    Uses :data:`~snowflake.connector._internal.api_client.client_api.async_core_driver`
    so telemetry RPCs do not block the event loop.
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle
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
        """Forward one caller-produced telemetry entry (e.g. Snowpark's) to sf_core.

        Core owns batching, flush threshold, and ``/telemetry/send`` egress.
        Raises :class:`~snowflake.connector.errors.InterfaceError` if the client
        has been closed; use :meth:`try_add_log_to_batch` for the fire-and-forget
        hot path.
        """
        if self._closed:
            raise InterfaceError(
                "Cannot add log to batch: TelemetryClient is closed. Obtain a fresh client from a new connection."
            )
        await async_core_driver.telemetry_send_log(
            conn_handle=self._conn_handle,
            message_json=json.dumps(telemetry_data.message),
            timestamp_ms=int(telemetry_data.timestamp),
        )

    async def try_add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Exception-swallowing wrapper over :meth:`add_log_to_batch` — the hot path."""
        try:
            await self.add_log_to_batch(telemetry_data)
        except Exception:
            logger.debug("Failed to add log to telemetry", exc_info=True)


# Must be the last statement; see ``install_backward_compatibility_getattr``.
install_backward_compatibility_getattr(__name__)
