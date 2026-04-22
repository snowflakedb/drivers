"""
PEP 249 Database API 2.0 Cursor Objects

This package defines the cursor classes as specified in PEP 249.

Hierarchy:
    SnowflakeCursorBase
    ├── SnowflakeCursor  — returns tuple rows
    └── DictCursor       — returns dict rows
"""

from __future__ import annotations

from typing import Union

from ._base import (
    DictRow,
    FetchMode,
    Row,
    SnowflakeCursorBase,
)
from ._dict_cursor import DictCursor
from ._result_metadata import QueryResultStats, ResultMetadata, ResultMetadataV2
from ._snowflake_cursor import SnowflakeCursor


CursorType = Union[type[SnowflakeCursor], type[DictCursor]]
CursorInstance = Union[SnowflakeCursor, DictCursor]

# Backward compatibility: async retry pattern used by Snowpark
ASYNC_RETRY_PATTERN = [1, 1, 2, 3, 4, 8, 10]


__all__ = [
    "ASYNC_RETRY_PATTERN",
    "CursorInstance",
    "CursorType",
    "DictCursor",
    "DictRow",
    "FetchMode",
    "QueryResultStats",
    "ResultMetadata",
    "ResultMetadataV2",
    "Row",
    "SnowflakeCursor",
    "SnowflakeCursorBase",
]
