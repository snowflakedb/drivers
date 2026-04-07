"""In-band telemetry client that sends events to sf_core via protobuf RPC.

All methods are fire-and-forget: failures are logged at DEBUG level and never
propagate to the caller.
"""

from __future__ import annotations

import logging

from typing import TYPE_CHECKING

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    TelemetrySendApiUsageRequest,
    TelemetrySendWrapperErrorRequest,
)


if TYPE_CHECKING:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
        ConnectionHandle,
        DatabaseDriverClient,
    )

logger = logging.getLogger(__name__)


class TelemetryClient:
    """Sends telemetry events to sf_core via protobuf RPC.

    Wrapper identity is registered by the Connection constructor via
    ``db_api.telemetry_init`` before connection_init. This client only sends
    runtime events — sf_core attaches the stored identity automatically.
    """

    def __init__(self, db_api: DatabaseDriverClient, conn_handle: ConnectionHandle) -> None:
        self._db_api = db_api
        self._conn_handle = conn_handle

    def send_api_usage(self, api_method: str) -> None:
        """Record an API method call for telemetry."""
        try:
            self._db_api.telemetry_send_api_usage(
                TelemetrySendApiUsageRequest(
                    conn_handle=self._conn_handle,
                    api_method=api_method,
                )
            )
        except Exception:
            logger.debug("Failed to send api_usage telemetry", exc_info=True)

    def send_wrapper_error(self, exception_type: str, error_source: str) -> None:
        """Record a wrapper error for telemetry."""
        try:
            self._db_api.telemetry_send_wrapper_error(
                TelemetrySendWrapperErrorRequest(
                    conn_handle=self._conn_handle,
                    exception_type=exception_type,
                    error_source=error_source,
                )
            )
        except Exception:
            logger.debug("Failed to send wrapper_error telemetry", exc_info=True)
