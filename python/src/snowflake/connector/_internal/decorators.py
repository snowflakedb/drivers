"""Decorators for internal use."""

import functools

from contextvars import ContextVar
from typing import Any, Callable, TypeVar, cast


F = TypeVar("F", bound=Callable[..., Any])


def internal_api(func: F) -> F:
    """
    Mark a method or function as internal.
    This is an identity function that returns the function unchanged.
    It serves as a marker for internal APIs that should not be used by external consumers.
    Args:
        func: The function or method to mark as internal
    Returns:
        The unchanged function
    """
    return func


def backward_compatibility(func: F) -> F:
    """Mark a method as backward compatibility utility"""
    return func


def pep249(func: F) -> F:
    """Mark a method or property as defined by PEP 249 (required or optional)."""
    return func


_TRACKING = ContextVar("_api_tracking", default=True)
_REPORTED_ERROR = ContextVar("_reported_error", default=0)


def _resolve_telemetry_client(self: Any) -> Any:
    """Resolve the TelemetryClient from a Connection or Cursor instance."""
    from snowflake.connector.connection import Connection
    from snowflake.connector.cursor._base import SnowflakeCursorBase

    if isinstance(self, Connection):
        return self._telemetry_client
    elif isinstance(self, SnowflakeCursorBase):
        return self._connection._telemetry_client
    return None


def api_telemetry(func: F) -> F:
    """Record api_usage and wrapper_error telemetry for public methods.

    **api_usage**: recorded once for the outermost user-initiated call.
    Nested decorated calls (e.g. ``commit`` -> ``cursor`` -> ``execute``)
    are suppressed via a ``ContextVar`` flag.

    **wrapper_error**: recorded once for the innermost decorated method
    where the exception originates.  A second ``ContextVar`` stores the
    ``id()`` of the already-reported exception so outer methods skip it.

    The ``api_method`` / ``error_source`` string is derived at runtime as
    ``"{ClassName}.{method_name}"``.
    """

    @functools.wraps(func)
    def wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
        track = _TRACKING.get()
        api_name = f"{type(self).__name__}.{func.__name__}"

        if track:
            client = _resolve_telemetry_client(self)
            if client is not None:
                client.send_api_usage(api_name)
            token = _TRACKING.set(False)

        try:
            return func(self, *args, **kwargs)
        except Exception as exc:
            if id(exc) != _REPORTED_ERROR.get():
                client = _resolve_telemetry_client(self)
                if client is not None:
                    client.send_wrapper_error(type(exc).__name__, api_name)
                _REPORTED_ERROR.set(id(exc))
            raise
        finally:
            if track:
                _TRACKING.reset(token)

    return cast(F, wrapper)
