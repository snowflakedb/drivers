"""Cursor precondition decorators shared by sync and async cursor implementations."""

from __future__ import annotations

from typing import Any

from ...errors import InterfaceError
from ..decorators import wrap_method_with_awaitable_pre, wrap_method_with_sync_pre
from ..errorcode import ER_CURSOR_IS_CLOSED
from .cursor_types import F


def _raise_if_cursor_closed(self: Any) -> None:
    if self.is_closed():
        raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)


def _raise_if_cursor_flag_closed(self: Any) -> None:
    if self._closed:
        raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)


def _run_prefetch_hook(self: Any) -> None:
    if self._prefetch_hook is not None:
        self._prefetch_hook()


async def _run_prefetch_hook_async(self: Any) -> None:
    if self._prefetch_hook is not None:
        await self._prefetch_hook()


def requires_open(func: F) -> F:
    """Reject the call when the cursor or its connection is closed."""
    return wrap_method_with_sync_pre(func, pre=_raise_if_cursor_closed)


def requires_open_cursor_not_connection(func: F) -> F:
    """Guard that only checks ``self._closed``, ignoring the connection state.

    Unlike ``requires_open`` (which delegates to ``is_closed()`` and therefore
    also rejects cursors whose *connection* has been closed), this decorator
    deliberately skips the connection check.  This preserves backward
    compatibility with the old driver, where fetch methods on a cursor with
    already-buffered results still worked after ``connection.close()``.
    """
    return wrap_method_with_sync_pre(func, pre=_raise_if_cursor_flag_closed)


def with_prefetch_hook(func: F) -> F:
    """Invoke the cursor's prefetch hook (if set) before entering the wrapped method."""
    return wrap_method_with_awaitable_pre(
        func,
        sync_pre=_run_prefetch_hook,
        async_pre=_run_prefetch_hook_async,
    )
