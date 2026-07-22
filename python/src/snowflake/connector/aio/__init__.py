"""Async PEP 249 entry points."""

from __future__ import annotations

from functools import update_wrapper
from typing import Any

from .._internal.decorators import awaitable_context_manager, pep249
from .connection import Connection, SnowflakeConnection
from .cursor import DictCursor, SnowflakeCursor
from .pandas_tools import write_pandas


@pep249
@awaitable_context_manager
async def connect(**kwargs: Any) -> Connection:
    """Create and open a connection to the database.

    Supports both ``conn = await connect(...)`` and
    ``async with connect(...) as conn:``.
    """
    conn = Connection(**kwargs)
    await conn.connect()
    return conn


update_wrapper(connect, Connection.__init__)


__all__ = [
    "Connection",
    "SnowflakeConnection",
    "connect",
    "DictCursor",
    "SnowflakeCursor",
    "write_pandas",
]
