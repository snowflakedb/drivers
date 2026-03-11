"""BACKWARD COMPATIBILITY MODULE ONLY"""

from enum import Enum


class TelemetryData:
    def __init__(self, message: str, timestamp: int) -> None:
        self.message = message
        self.timestamp = timestamp


class TelemetryField(Enum):
    KEY_SOURCE = "source"
    KEY_TYPE = "type"


class TelemetryClient:
    def try_add_log_to_batch(self, *args, **kwargs):  # type: ignore
        raise NotImplementedError()
