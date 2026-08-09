"""Connection precondition decorators shared by sync and async connection implementations."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ...errors import DatabaseError
from ..decorators import wrap_method_with_sync_pre
from ..errorcode import ER_CONNECTION_IS_CLOSED
from ..sqlstate import SQLSTATE_CONNECTION_NOT_EXISTS
from .connection_types import F


if TYPE_CHECKING:
    from ...aio.connection import Connection as AsyncConnection
    from ...connection import Connection


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
    return wrap_method_with_sync_pre(func, pre=_raise_if_connection_closed)
