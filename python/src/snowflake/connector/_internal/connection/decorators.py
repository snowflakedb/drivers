"""Connection precondition decorators shared by sync and async connection implementations."""

from __future__ import annotations

import functools
import inspect

from collections.abc import Awaitable, Callable
from typing import TYPE_CHECKING, Any, cast

from ...errors import DatabaseError
from ..errorcode import ER_CONNECTION_IS_CLOSED
from ..sqlstate import SQLSTATE_CONNECTION_NOT_EXISTS
from .connection_types import F


if TYPE_CHECKING:
    from ...connection import Connection


def _wrap_connection_method(
    func: F,
    *,
    sync_pre: Callable[[Any], None],
    async_pre: Callable[[Any], Awaitable[None]],
) -> F:
    """Wrap *func* with sync/async pre-call hooks."""
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


def _raise_if_connection_closed(self: Connection) -> None:
    # TODO: it should rather raise InterfaceError, consider to align with the cursor
    if self.is_closed():
        raise DatabaseError(
            msg="Connection is closed.",
            errno=ER_CONNECTION_IS_CLOSED,
            sqlstate=SQLSTATE_CONNECTION_NOT_EXISTS,
        )


async def _raise_if_connection_closed_async(self: Any) -> None:
    if self.conn_handle is None or await self.is_closed():
        raise DatabaseError(
            msg="Connection is closed.",
            errno=ER_CONNECTION_IS_CLOSED,
            sqlstate=SQLSTATE_CONNECTION_NOT_EXISTS,
        )


def requires_open(func: F) -> F:
    """Raise ``DatabaseError`` if the connection is closed."""
    return _wrap_connection_method(
        func,
        sync_pre=_raise_if_connection_closed,
        async_pre=_raise_if_connection_closed_async,
    )
