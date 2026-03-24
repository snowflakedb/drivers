"""Constants module for snowflake-connector-python."""

from enum import Enum, unique

from .config_manager import CONFIG_FILE, CONNECTIONS_FILE  # noqa: F401 - backward compatibility re-exports


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
