"""PEP 249 error-handler routing via ``ErrorHandlerMixin``.

How to raise errors in the driver:
- Classes (Connection, Cursor, ResultBatch): inherit ``ErrorHandlerMixin``.
  Public methods are wrapped automatically via ``__init_subclass__``.
  Then use plain ``raise``.
- Free functions with a ``conn`` argument (e.g. ``write_pandas``): wrap the body
  in ``try/except Error`` and call ``route_exception(conn, None, exc)``.
"""

from __future__ import annotations

import functools
import inspect

from collections.abc import Callable
from contextvars import ContextVar
from typing import TYPE_CHECKING, Any, NoReturn

from ..errors import Error


if TYPE_CHECKING:
    from ..connection import Connection
    from ..cursor import SnowflakeCursorBase


def route_exception(
    connection: Connection | None,
    cursor: SnowflakeCursorBase | None,
    exc: Error,
) -> NoReturn:
    """Route an ``Error`` through the PEP 249 errorhandler chain, then re-raise.

    The handler may observe, log, or replace the error (by raising a different
    exception), but it cannot suppress it — the original is always re-raised.
    """
    error_value = _error_to_value(exc)
    Error.hand_to_other_handler(connection, cursor, type(exc), error_value)
    raise exc


class ErrorHandlerMixin:
    """Mixin that routes ``Error`` exceptions from public methods through PEP 249 errorhandler.

    Inherit this class and override ``_errorhandler_connection`` and/or
    ``_errorhandler_cursor`` to supply context.  Public methods are wrapped
    automatically at class creation time via ``__init_subclass__``.
    """

    def __init_subclass__(cls, **kwargs: Any) -> None:
        super().__init_subclass__(**kwargs)
        _apply_errorhandler(cls)

    @property
    def _errorhandler_connection(self) -> Connection | None:
        return None

    @property
    def _errorhandler_cursor(self) -> SnowflakeCursorBase | None:
        return None


def _apply_errorhandler(cls: type) -> None:
    """Wrap public methods of *cls* with error-handler routing."""
    for name in list(vars(cls)):
        # private/dunder — not part of the public API surface
        if name.startswith("_"):
            continue
        attr = vars(cls)[name]
        # descriptors — not regular instance methods
        if isinstance(attr, (property, classmethod, staticmethod)):
            continue
        # class-level constants or non-callable attributes
        if not callable(attr):
            continue
        # generators yield lazily; errors surface at iteration time, not call time
        if inspect.isgeneratorfunction(attr):
            continue
        setattr(cls, name, _wrap_method(attr))


# Prevents double-routing when a wrapped public method calls another.
# Global (not per-object):
# if conn A's method somehow triggers conn B's method in the same context, conn B's errors won't be routed.
# This is acceptable because a connection method should never call into another connection
# and per-object tracking would add overhead on every call for a scenario that should not occur.
_errorhandler_active: ContextVar[bool] = ContextVar("_errorhandler_active", default=False)


def _wrap_method(method: Callable) -> Callable:
    """Wrap a method to route ``Error`` through the errorhandler chain."""

    @functools.wraps(method)
    def wrapper(self: ErrorHandlerMixin, *args: Any, **kwargs: Any) -> Any:
        if _errorhandler_active.get():
            return method(self, *args, **kwargs)
        token = _errorhandler_active.set(True)
        try:
            return method(self, *args, **kwargs)
        except Error as exc:
            route_exception(
                self._errorhandler_connection,
                self._errorhandler_cursor,
                exc,
            )
        finally:
            _errorhandler_active.reset(token)

    return wrapper


def _error_to_value(exc: Error) -> dict[str, Any]:
    """Deconstruct an ``Error`` into the dict expected by ``hand_to_other_handler``."""
    return {
        "msg": exc.raw_msg,
        "errno": exc.errno,
        "sqlstate": exc.sqlstate,
        "sfqid": exc.sfqid,
        "query": exc.query,
    }
