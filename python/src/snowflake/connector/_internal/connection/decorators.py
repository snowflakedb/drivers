"""Connection precondition decorators shared by sync and async connection implementations."""

from __future__ import annotations

import functools
import inspect

from collections.abc import Callable
from typing import TYPE_CHECKING, Any, cast

from ...errors import DatabaseError
from ..errorcode import ER_CONNECTION_IS_CLOSED
from ..sqlstate import SQLSTATE_CONNECTION_NOT_EXISTS
from .connection_types import F


if TYPE_CHECKING:
    from ...aio.connection import Connection as AsyncConnection
    from ...connection import Connection


def _wrap_connection_method(
    func: F,
    *,
    pre: Callable[[Any], None],
) -> F:
    """Wrap *func* with a sync pre-call hook (used for both sync and async methods)."""
    if inspect.iscoroutinefunction(func):

        @functools.wraps(func)
        async def async_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
            pre(self)
            return await func(self, *args, **kwargs)

        return cast(F, async_wrapper)

    @functools.wraps(func)
    def sync_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
        pre(self)
        return func(self, *args, **kwargs)

    return cast(F, sync_wrapper)


def _raise_if_connection_closed(self: Connection | AsyncConnection) -> None:
    # TODO: it should rather raise InterfaceError, consider to align with the cursor
    if self.conn_handle is None or self.is_closed():
        raise DatabaseError(
            msg="Connection is closed.",
            errno=ER_CONNECTION_IS_CLOSED,
            sqlstate=SQLSTATE_CONNECTION_NOT_EXISTS,
        )


def requires_open(func: F) -> F:
    """Raise ``DatabaseError`` if the connection is closed."""
    return _wrap_connection_method(func, pre=_raise_if_connection_closed)
