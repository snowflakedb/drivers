"""PEP 249 error-handler routing via ``ErrorHandlerMixin``.

How to raise errors in the driver:
- Classes (Connection, Cursor, ResultBatch): inherit ``ErrorHandlerMixin``.
  Public methods are wrapped automatically via ``__init_subclass__``.
  Then use plain ``raise``.
- Free functions with a ``conn`` argument (e.g. ``write_pandas``): wrap the body
  in ``try/except Error`` and call ``route_exception(conn, None, exc)``.

This wrapping is also where ``wrapper_error`` telemetry is collected (see
``_report_wrapper_error`` / ``_report_wrapper_error_async``): every wrapped
call that catches an exception reports it, tagged with its own method name,
but only the outermost frame in a call chain hands the error to the PEP 249
errorhandler, so PEP 249 routing still fires exactly once.
"""

from __future__ import annotations

import functools
import inspect

from collections.abc import Callable
from contextvars import ContextVar
from typing import TYPE_CHECKING, Any, NoReturn

from ..errors import Error
from .decorators import _schedule_async_telemetry, _telemetry_client_for
from .logging import get_logger


if TYPE_CHECKING:
    from ..aio.connection import Connection as AsyncConnection
    from ..aio.cursor import SnowflakeCursorBase as AsyncSnowflakeCursorBase
    from ..connection import Connection
    from ..cursor import SnowflakeCursorBase

logger = get_logger(__name__)


def route_exception(
    connection: Connection | AsyncConnection | None,
    cursor: SnowflakeCursorBase | AsyncSnowflakeCursorBase | None,
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
    def _errorhandler_connection(self) -> Connection | AsyncConnection | None:
        return None

    @property
    def _errorhandler_cursor(self) -> SnowflakeCursorBase | AsyncSnowflakeCursorBase | None:
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
        # generators/async generators yield lazily; errors surface at iteration
        # time, not call time — the inner calls they drive are wrapped individually.
        if inspect.isgeneratorfunction(attr) or inspect.isasyncgenfunction(attr):
            continue
        if inspect.iscoroutinefunction(attr):
            setattr(cls, name, _wrap_async_method(attr))
        else:
            setattr(cls, name, _wrap_method(attr))


# Prevents double-routing when a wrapped public method calls another.
# Global (not per-object):
# if conn A's method somehow triggers conn B's method in the same context, conn B's errors won't be routed.
# This is acceptable because a connection method should never call into another connection
# and per-object tracking would add overhead on every call for a scenario that should not occur.
_errorhandler_active: ContextVar[bool] = ContextVar("_errorhandler_active", default=False)


def _report_wrapper_error(self: Any, method: Callable[..., Any], exc: BaseException) -> None:
    """Send ``wrapper_error`` telemetry for *exc*."""
    try:
        # circular-import: telemetry.py imports errorhandler helpers; defer this import
        # to call time so the module-level import graph doesn't cycle.
        from .telemetry import AsyncTelemetryClient

        error_source = f"{type(self).__name__}.{method.__name__}"
        client = _telemetry_client_for(self)
        if client is None:
            return
        if isinstance(client, AsyncTelemetryClient):
            # A sync method on an async-flavored class (e.g. a non-coroutine method on
            # AsyncConnection/AsyncCursorBase) still carries an AsyncTelemetryClient.
            _schedule_async_telemetry(client.send_wrapper_error(type(exc).__name__, error_source))
        else:
            client.send_wrapper_error(type(exc).__name__, error_source)
    except Exception:
        logger.debug("Failed to send wrapper_error telemetry")


async def _report_wrapper_error_async(self: Any, method: Callable[..., Any], exc: BaseException) -> None:
    """Async counterpart of :func:`_report_wrapper_error`."""
    try:
        # circular-import: telemetry.py imports errorhandler helpers; defer this import
        # to call time so the module-level import graph doesn't cycle.
        from .telemetry import AsyncTelemetryClient

        error_source = f"{type(self).__name__}.{method.__name__}"
        client = _telemetry_client_for(self)
        if client is None:
            return
        if isinstance(client, AsyncTelemetryClient):
            await client.send_wrapper_error(type(exc).__name__, error_source)
        else:
            client.send_wrapper_error(type(exc).__name__, error_source)
    except Exception:
        logger.debug("Failed to send wrapper_error telemetry")


def _wrap_method(method: Callable) -> Callable:
    """Wrap a method to report ``wrapper_error`` telemetry and route ``Error`` through the errorhandler chain."""

    @functools.wraps(method)
    def wrapper(self: ErrorHandlerMixin, *args: Any, **kwargs: Any) -> Any:
        nested = _errorhandler_active.get()
        if not nested:
            token = _errorhandler_active.set(True)
        try:
            return method(self, *args, **kwargs)
        except Exception as exc:
            _report_wrapper_error(self, method, exc)
            if not nested and isinstance(exc, Error):
                route_exception(
                    self._errorhandler_connection,
                    self._errorhandler_cursor,
                    exc,
                )
            raise
        finally:
            if not nested:
                _errorhandler_active.reset(token)

    return wrapper


def _wrap_async_method(method: Callable) -> Callable:
    """Async counterpart of :func:`_wrap_method`."""

    @functools.wraps(method)
    async def wrapper(self: ErrorHandlerMixin, *args: Any, **kwargs: Any) -> Any:
        nested = _errorhandler_active.get()
        if not nested:
            token = _errorhandler_active.set(True)
        try:
            return await method(self, *args, **kwargs)
        except Exception as exc:
            await _report_wrapper_error_async(self, method, exc)
            if not nested and isinstance(exc, Error):
                route_exception(
                    self._errorhandler_connection,
                    self._errorhandler_cursor,
                    exc,
                )
            raise
        finally:
            if not nested:
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
        "request_id": exc.request_id,
        "parameter": exc.parameter,
        "validation_code": exc.validation_code,
    }
