"""BACKWARD COMPATIBILITY MODULE ONLY"""

import warnings

from enum import Enum

from ._internal.backward_compatibility import _is_caller_external


class TelemetryData:
    TRUE = 1  # snowpark_compat
    FALSE = 0  # snowpark_compat

    def __init__(self, message: str, timestamp: int) -> None:
        self.message = message
        self.timestamp = timestamp


class TelemetryField(Enum):
    KEY_SOURCE = "source"
    KEY_TYPE = "type"


_TRY_ADD_LOG_TO_BATCH_WARNED = False


class TelemetryClient:
    def try_add_log_to_batch(self, *args, **kwargs):  # type: ignore
        global _TRY_ADD_LOG_TO_BATCH_WARNED
        if not _TRY_ADD_LOG_TO_BATCH_WARNED and _is_caller_external():
            _TRY_ADD_LOG_TO_BATCH_WARNED = True
            warnings.warn(
                "'snowflake.connector.telemetry.TelemetryClient.try_add_log_to_batch' "
                "is retained only for backward compatibility and is not used by the "
                "Universal Driver; use 'snowflake.connector._common.telemetry' instead. "
                "It may be removed in a future release.",
                DeprecationWarning,
                stacklevel=2,
            )
        raise NotImplementedError()
