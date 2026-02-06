"""
Constants for the Snowflake connector.

This module defines constants used throughout the Snowflake connector.
"""

from enum import Enum


class QueryStatus(Enum):
    """Query status enumeration matching Snowflake query status values."""

    RUNNING = 0
    ABORTING = 1
    SUCCESS = 2
    FAILED_WITH_ERROR = 3
    ABORTED = 4
    QUEUED = 5
    FAILED_WITH_INCIDENT = 6
    DISCONNECTED = 7
    RESUMING_WAREHOUSE = 8
    QUEUED_REPARING_WAREHOUSE = 9  # Note: typo is intentional from Java
    RESTARTED = 10
    BLOCKED = 11
    NO_DATA = 12
