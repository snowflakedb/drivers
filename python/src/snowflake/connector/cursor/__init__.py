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
from ._base import SnowflakeCursorBase
from ._dict_cursor import DictCursor
from ._snowflake_cursor import SnowflakeCursor


CursorType = type[SnowflakeCursor] | type[DictCursor]
CursorInstance = SnowflakeCursor | DictCursor

# Backward compatibility: async retry pattern used by Snowpark
ASYNC_RETRY_PATTERN = [1, 1, 2, 3, 4, 8, 10]


__all__ = [
    "ASYNC_RETRY_PATTERN",
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
