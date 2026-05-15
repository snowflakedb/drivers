"""Public telemetry API.

Consumers (snowflake-cli, snowflake-sqlalchemy, snowpy) use this module to
send custom telemetry logs via ``connection._telemetry``.

Buffering and export happen in the Rust core; this wrapper simply forwards
calls through the protobuf RPC layer.
"""

from __future__ import annotations

import logging
import time

from enum import Enum, unique
from typing import Any

from ._internal.telemetry import CoreTelemetryClient


logger = logging.getLogger(__name__)


@unique
class TelemetryField(Enum):
    """Minimal set of telemetry field keys used by downstream consumers."""

    KEY_SOURCE = "source"
    KEY_TYPE = "type"
    KEY_VALUE = "value"


class TelemetryData:
    """A single telemetry log entry (message dict + epoch-ms timestamp)."""

    TRUE = 1
    FALSE = 0

    def __init__(self, message: dict[str, Any], timestamp: int) -> None:
        self.message = message
        self.timestamp = timestamp

    @classmethod
    def from_telemetry_data_dict(
        cls,
        *,
        from_dict: dict[str, Any] | None = None,
        timestamp: int | None = None,
        connection: Any | None = None,
    ) -> TelemetryData:
        """Build a TelemetryData from a dict, adding ``source`` from the connection if absent."""
        msg = dict(from_dict) if from_dict else {}

        if connection is not None and TelemetryField.KEY_SOURCE.value not in msg:
            app = getattr(connection, "application", None)
            if app:
                msg[TelemetryField.KEY_SOURCE.value] = app

        return cls(
            message=msg,
            timestamp=timestamp if timestamp is not None else int(time.time() * 1000),
        )

    def to_dict(self) -> dict[str, Any]:
        return {"message": self.message, "timestamp": str(self.timestamp)}

    def __repr__(self) -> str:
        return f"TelemetryData(message={self.message!r}, timestamp={self.timestamp})"


class TelemetryClient:
    """Public telemetry client exposed via ``connection._telemetry``.

    Delegates to a shared :class:`CoreTelemetryClient` so that
    ``connection.telemetry_enabled = False`` disables both user-facing and
    internal (api_telemetry) telemetry in one shot.

    Buffering and flush are owned by the Rust core.
    """

    def __init__(
        self,
        core: CoreTelemetryClient | None = None,
        **kwargs: Any,
    ) -> None:
        self._core = core

    def add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Send a single telemetry log to the core for buffering."""
        if self._core is None:
            return
        self._core.send_user_log(telemetry_data)

    def try_add_log_to_batch(self, telemetry_data: TelemetryData) -> None:
        """Like :meth:`add_log_to_batch` but swallows exceptions."""
        try:
            self.add_log_to_batch(telemetry_data)
        except Exception:
            logger.debug("Failed to add telemetry log to batch", exc_info=True)

    def send_batch(self) -> None:
        """Explicitly flush all buffered telemetry for this session."""
        if self._core is None:
            return
        self._core.flush()

    def is_enabled(self) -> bool:
        return self._core is not None and self._core.enabled
