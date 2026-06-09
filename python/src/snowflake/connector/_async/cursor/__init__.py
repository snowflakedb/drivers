"""
PEP 249 Database API 2.0 Cursor Objects (async)

This package defines the async cursor classes as specified in PEP 249.

Hierarchy:
    AsyncSnowflakeCursorBase
    ├── AsyncSnowflakeCursor  — returns tuple rows
    └── AsyncDictCursor       — returns dict rows
"""

from __future__ import annotations

from ..._internal.cursor import DictRow, QueryResultStats, ResultMetadata, ResultMetadataV2, Row
from ._base import AsyncSnowflakeCursorBase
from ._dict_cursor import AsyncDictCursor
from ._snowflake_cursor import AsyncSnowflakeCursor


AsyncCursorType = type[AsyncSnowflakeCursor] | type[AsyncDictCursor]
AsyncCursorInstance = AsyncSnowflakeCursor | AsyncDictCursor


__all__ = [
    "AsyncCursorInstance",
    "AsyncCursorType",
    "AsyncDictCursor",
    "DictRow",
    "QueryResultStats",
    "ResultMetadata",
    "ResultMetadataV2",
    "Row",
    "AsyncSnowflakeCursor",
    "AsyncSnowflakeCursorBase",
]
