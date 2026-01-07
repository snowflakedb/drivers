"""BACKWARD COMPATIBILITY MODULE ONLY"""

import time

from snowflake.connector._internal import internal


@internal
def get_time_millis() -> int:
    """Return the current time in milliseconds."""
    return int(time.time() * 1000)
