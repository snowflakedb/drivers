"""Type definitions for performance tests"""
from enum import Enum


class TestType(str, Enum):
    """Enum for test types"""
    SELECT = "select"
    PUT_GET = "put_get"

