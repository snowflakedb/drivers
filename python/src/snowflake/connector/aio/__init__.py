"""Async-native Snowflake connector interfaces.

This package mirrors the synchronous ``snowflake.connector`` hierarchy
but uses ``async def`` throughout.
"""

from __future__ import annotations

from typing import Any

from ..connection_config import ConnectionConfig
from ._result_batch import AsyncResultBatch
from .connection import AsyncConnection
from .cursor import AsyncDictCursor, AsyncSnowflakeCursor, AsyncSnowflakeCursorBase


async def connect(
    *,
    connection_name: str | None = None,
    connections_file_path: str | None = None,
    config: ConnectionConfig | None = None,
    **kwargs: Any,
) -> AsyncConnection:
    """Create an async connection to Snowflake.

    This is the async counterpart to :func:`snowflake.connector.connect`.
    """
    conn = AsyncConnection(
        connection_name=connection_name,
        connections_file_path=connections_file_path,
        config=config,
        **kwargs,
    )
    await conn.connect()
    return conn


__all__ = [
    "AsyncConnection",
    "AsyncDictCursor",
    "AsyncResultBatch",
    "AsyncSnowflakeCursor",
    "AsyncSnowflakeCursorBase",
    "connect",
]
