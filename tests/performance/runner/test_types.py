"""Type definitions for performance tests"""
from enum import Enum


class PerfTestType(str, Enum):
    """Enum for test types"""
    SELECT = "select"
    PUT_GET = "put_get"
    SELECT_RECORDED_HTTP = "select_recorded_http"
    COLD_START = "cold_start"
    COLD_START_RECORDED_HTTP = "cold_start_recorded_http"
    CONCURRENT = "concurrent"
