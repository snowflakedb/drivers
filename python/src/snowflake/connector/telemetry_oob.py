"""BACKWARD COMPATIBILITY MODULE ONLY — out-of-band telemetry stub."""

from __future__ import annotations

from queue import Queue
from threading import Lock
from typing import Any


class TelemetryService:
    """Singleton stub for the out-of-band telemetry service.

    The Universal Driver does not implement OOB telemetry. This stub keeps
    Snowpark mock/ imports working without errors, but all telemetry calls
    are silently dropped.
    """

    __instance: TelemetryService | None = None
    __lock_init: Lock = Lock()

    @classmethod
    def get_instance(cls) -> TelemetryService:
        with cls.__lock_init:
            if cls.__instance is None:
                cls.__instance = cls.__new__(cls)
                cls.__instance._init()
        return cls.__instance

    def _init(self) -> None:
        self._enabled = False
        self._queue: Queue[Any] = Queue()

    def __init__(self) -> None:
        if TelemetryService.__instance is not None and TelemetryService.__instance is not self:
            raise RuntimeError("TelemetryService is a singleton — use get_instance()")
        self._init()

    @property
    def enabled(self) -> bool:
        return False

    def enable(self) -> None:
        pass

    def disable(self) -> None:
        pass

    @property
    def queue(self) -> Queue[Any]:
        return self._queue

    def add_log_to_batch(self, telemetry_data: Any) -> None:
        pass

    def report_client_failure_event(self, *args: Any, **kwargs: Any) -> None:
        pass

    def flush_batch(self, *args: Any, **kwargs: Any) -> None:
        pass

    def close(self, *args: Any, **kwargs: Any) -> None:
        pass
