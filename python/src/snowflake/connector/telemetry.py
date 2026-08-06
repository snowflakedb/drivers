"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from enum import Enum
from typing import Any


class TelemetryData:
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


class TelemetryField(Enum):
    KEY_SOURCE = "source"
    KEY_TYPE = "type"
    KEY_SFQID = "query_id"
    KEY_VALUE = "value"
