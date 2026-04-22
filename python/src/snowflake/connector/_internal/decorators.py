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


def api_telemetry(func: F) -> F:
    """Send api_usage telemetry on the first public-method entry.

    When the decorated method is called and tracking is enabled (the default),
    record the call via :pymeth:`TelemetryClient.send_api_usage` and then
    *disable* tracking for the duration of the method body.  Any nested
    decorated calls (e.g. ``commit`` -> ``cursor`` -> ``execute``) are
    therefore suppressed automatically, ensuring only the outermost
    user-initiated call is recorded.

    The ``api_method`` string is derived at runtime as
    ``"{ClassName}.{method_name}"``.
    """

    @functools.wraps(func)
    def wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
        if _TRACKING.get():
            from snowflake.connector.connection import Connection
            from snowflake.connector.cursor._base import SnowflakeCursorBase

            api_name = f"{type(self).__name__}.{func.__name__}"
            if isinstance(self, Connection):
                self._telemetry_client.send_api_usage(api_name)
            elif isinstance(self, SnowflakeCursorBase):
                self._connection._telemetry_client.send_api_usage(api_name)

            token = _TRACKING.set(False)
            try:
                return func(self, *args, **kwargs)
            finally:
                _TRACKING.reset(token)
        return func(self, *args, **kwargs)

    return cast(F, wrapper)
