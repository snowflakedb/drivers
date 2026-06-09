"""Cursor precondition decorators shared by sync and async cursor implementations."""

from __future__ import annotations

import functools
import inspect

from collections.abc import Awaitable, Callable
from typing import Any, cast

from ...errors import InterfaceError
from ..errorcode import ER_CURSOR_IS_CLOSED
from .cursor_types import F


def _wrap_cursor_method(
    func: F,
    *,
    sync_pre: Callable[[Any], None],
    async_pre: Callable[[Any], Awaitable[None]],
) -> F:
    """Wrap *func* with sync/async pre-call hooks."""
    if inspect.isasyncgenfunction(func):

        @functools.wraps(func)
        async def gen_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
            await async_pre(self)
            async for value in func(self, *args, **kwargs):
                yield value

        return cast(F, gen_wrapper)

    if inspect.iscoroutinefunction(func):

        @functools.wraps(func)
        async def async_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
            await async_pre(self)
            return await func(self, *args, **kwargs)

        return cast(F, async_wrapper)

    @functools.wraps(func)
    def sync_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
        sync_pre(self)
        return func(self, *args, **kwargs)

    return cast(F, sync_wrapper)


def _raise_if_cursor_closed(self: Any) -> None:
    if self.is_closed():
        raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)


async def _raise_if_cursor_closed_async(self: Any) -> None:
    if await self.is_closed():
        raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)


def _raise_if_cursor_flag_closed(self: Any) -> None:
    if self._closed:
        raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)


async def _raise_if_cursor_flag_closed_async(self: Any) -> None:
    _raise_if_cursor_flag_closed(self)


def _run_prefetch_hook(self: Any) -> None:
    if self._prefetch_hook is not None:
        self._prefetch_hook()


async def _run_prefetch_hook_async(self: Any) -> None:
    if self._prefetch_hook is not None:
        await self._prefetch_hook()


def requires_open(func: F) -> F:
    """Reject the call when the cursor or its connection is closed."""
    return _wrap_cursor_method(
        func,
        sync_pre=_raise_if_cursor_closed,
        async_pre=_raise_if_cursor_closed_async,
    )


def requires_open_cursor_not_connection(func: F) -> F:
    """Guard that only checks ``self._closed``, ignoring the connection state.

    Unlike ``_requires_open`` (which delegates to ``is_closed()`` and therefore
    also rejects cursors whose *connection* has been closed), this decorator
    deliberately skips the connection check.  This preserves backward
    compatibility with the old driver, where fetch methods on a cursor with
    already-buffered results still worked after ``connection.close()``.

    The ``self._closed`` check is a cheap synchronous flag read; async wrappers
    are coroutines purely so they compose with other async cursor decorators.
    """
    return _wrap_cursor_method(
        func,
        sync_pre=_raise_if_cursor_flag_closed,
        async_pre=_raise_if_cursor_flag_closed_async,
    )


def with_prefetch_hook(func: F) -> F:
    """Invoke the cursor's prefetch hook (if set) before entering the wrapped method."""
    return _wrap_cursor_method(
        func,
        sync_pre=_run_prefetch_hook,
        async_pre=_run_prefetch_hook_async,
    )
