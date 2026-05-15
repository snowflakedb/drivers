"""In-band telemetry functions that send events to sf_core via protobuf RPC.

All functions are fire-and-forget: failures are logged at DEBUG level and never
propagate to the caller.
"""

from __future__ import annotations

import json
import logging

from typing import TYPE_CHECKING

from .api_client.client_api import core_driver


if TYPE_CHECKING:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
        ConnectionHandle,
    )
    from snowflake.connector.telemetry import TelemetryData

logger = logging.getLogger(__name__)


class CoreTelemetryClient:
    """Internal telemetry client used by both the public TelemetryClient and
    the api_telemetry decorator. Respects a single ``_enabled`` flag so that
    ``connection.telemetry_enabled = False`` disables all telemetry.

    Server-side disablement (``CLIENT_TELEMETRY_ENABLED=false``) is handled
    entirely by the Rust core which skips session registration — RPCs from
    Python become cheap no-ops. The Python flag only serves the explicit
    programmatic disable use-case, avoiding unnecessary RPC overhead.
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle
        self._enabled = True

    @property
    def enabled(self) -> bool:
        return self._enabled

    @enabled.setter
    def enabled(self, value: bool) -> None:
        self._enabled = value

    def send_api_usage(self, api_method: str) -> None:
        if not self._enabled:
            return
        try:
            core_driver.telemetry_send_api_usage(
                conn_handle=self._conn_handle,
                api_method=api_method,
            )
        except Exception:
            logger.debug("Failed to send api_usage telemetry", exc_info=True)

    def send_wrapper_error(self, exception_type: str, error_source: str) -> None:
        if not self._enabled:
            return
        try:
            core_driver.telemetry_send_wrapper_error(
                conn_handle=self._conn_handle,
                exception_type=exception_type,
                error_source=error_source,
            )
        except Exception:
            logger.debug("Failed to send wrapper_error telemetry", exc_info=True)

    def send_user_log(self, telemetry_data: TelemetryData) -> None:
        if not self._enabled:
            return
        try:
            core_driver.telemetry_send_json(
                conn_handle=self._conn_handle,
                entry_json=json.dumps(telemetry_data.to_dict()),
            )
        except Exception:
            logger.debug("Failed to send telemetry json", exc_info=True)

    def flush(self) -> None:
        if not self._enabled:
            return
        try:
            core_driver.telemetry_flush(conn_handle=self._conn_handle)
        except Exception:
            logger.debug("Failed to flush telemetry", exc_info=True)
