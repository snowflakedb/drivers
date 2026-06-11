"""Async PEP 249 entry points."""

from __future__ import annotations

from functools import wraps
from typing import Any

from .._internal.decorators import awaitable_context_manager, pep249
from .connection import AsyncConnection


@pep249
@awaitable_context_manager
@wraps(AsyncConnection.__init__)
async def connect_async(**kwargs: Any) -> AsyncConnection:
    """Create and open a connection to the database.

    Supports both ``conn = await connect_async(...)`` and
    ``async with connect_async(...) as conn:``.
    """
    conn = AsyncConnection(**kwargs)
    await conn.connect()
    return conn


__all__ = [
    "AsyncConnection",
    "connect_async",
]
