"""
PEP 249 Database API 2.0 Cursor Objects

This package defines the cursor classes as specified in PEP 249.

Hierarchy:
    SnowflakeCursorBase
    ├── SnowflakeCursor  — returns tuple rows
    └── DictCursor       — returns dict rows
"""

from __future__ import annotations

from .._internal.cursor import DictRow, QueryResultStats, ResultMetadata, ResultMetadataV2, Row
from .._internal.cursor.query_result_waiter import _RETRY_PATTERN as ASYNC_RETRY_PATTERN
from ._base import SnowflakeCursorBase
from ._dict_cursor import DictCursor
from ._snowflake_cursor import SnowflakeCursor


CursorType = type[SnowflakeCursor] | type[DictCursor]
CursorInstance = SnowflakeCursor | DictCursor


__all__ = [
    "ASYNC_RETRY_PATTERN",  # snowpark_compat
    "CursorInstance",
    "CursorType",
    "DictCursor",
    "DictRow",
    "QueryResultStats",
    "ResultMetadata",
    "ResultMetadataV2",
    "Row",
    "SnowflakeCursor",
    "SnowflakeCursorBase",
]
