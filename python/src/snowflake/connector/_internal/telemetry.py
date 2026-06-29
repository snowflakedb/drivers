"""In-band telemetry client that sends events to sf_core via protobuf RPC.

All methods are fire-and-forget: failures are logged at DEBUG level and never
propagate to the caller.
"""

from __future__ import annotations

import logging

from typing import TYPE_CHECKING

from .api_client.client_api import async_core_driver, core_driver


if TYPE_CHECKING:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
        ConnectionHandle,
    )

logger = logging.getLogger(__name__)


class TelemetryClient:
    """Sends telemetry events to sf_core via protobuf RPC.

    Wrapper identity is passed as part of ``connection_init``. This client only
    sends runtime events — sf_core attaches the stored identity automatically.
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle

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


class AsyncTelemetryClient:
    """Async counterpart of :class:`TelemetryClient` for :class:`~snowflake.connector.aio.Connection`.

    Uses :data:`~snowflake.connector._internal.api_client.client_api.async_core_driver`
    so telemetry RPCs do not block the event loop.
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle

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
