"""Constants module for snowflake-connector-python."""

from enum import Enum, unique

from ._internal.type_codes import (
    ARRAY,
    BINARY,
    BOOLEAN,
    DATE,
    FIXED,
    GEOGRAPHY,
    GEOMETRY,
    OBJECT,
    REAL,
    TEXT,
    TIME,
    TIMESTAMP,
    TIMESTAMP_LTZ,
    TIMESTAMP_NTZ,
    TIMESTAMP_TZ,
    VARIANT,
    VECTOR,
)
from .config_manager import CONFIG_FILE, CONNECTIONS_FILE  # noqa: F401 - backward compatibility re-exports


FIELD_ID_TO_NAME = {
    FIXED: "FIXED",
    REAL: "REAL",
    TEXT: "TEXT",
    DATE: "DATE",
    TIMESTAMP: "TIMESTAMP",
    VARIANT: "VARIANT",
    TIMESTAMP_LTZ: "TIMESTAMP_LTZ",
    TIMESTAMP_TZ: "TIMESTAMP_TZ",
    TIMESTAMP_NTZ: "TIMESTAMP_NTZ",
    OBJECT: "OBJECT",
    ARRAY: "ARRAY",
    BINARY: "BINARY",
    TIME: "TIME",
    BOOLEAN: "BOOLEAN",
    GEOGRAPHY: "GEOGRAPHY",
    GEOMETRY: "GEOMETRY",
    VECTOR: "VECTOR",
}


@unique
class QueryStatus(Enum):
    RUNNING = 0
    ABORTING = 1
    SUCCESS = 2
    FAILED_WITH_ERROR = 3
    ABORTED = 4
    QUEUED = 5
    FAILED_WITH_INCIDENT = 6
    DISCONNECTED = 7
    RESUMING_WAREHOUSE = 8
    QUEUED_REPARING_WAREHOUSE = 9  # intentional typo, matches server-side QueryDTO.java
    RESTARTED = 10
    BLOCKED = 11
    NO_DATA = 12
