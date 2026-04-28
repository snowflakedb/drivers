"""Marker and annotation decorators for internal APIs.

The runtime machinery for ``@backward_compatibility`` (call-time wrapper,
module ``__getattr__`` installer, dedup state) lives in
:mod:`._internal.backward_compatibility`; this module is intentionally kept
to just the decorator façade.
"""

from __future__ import annotations

import functools
import inspect

from contextvars import ContextVar
from typing import Any, Callable, TypeVar, cast

from .backward_compatibility import apply_backward_compatibility


F = TypeVar("F", bound=Callable[..., Any])

# Generic pass-through: functions, classes, and descriptors all round-trip
# through ``@backward_compatibility`` as the same logical type (a class stays
# the same class; a wrapped function preserves its signature via
# ``functools.wraps``). Using ``TypeVar("T")`` lets mypy preserve the input
# type across the decorator instead of widening it to ``Any``.
T = TypeVar("T")


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


def pep249(func: F) -> F:
    """Mark a method or property as defined by PEP 249 (required or optional)."""
    return func


def backward_compatibility(obj: T) -> T:
    """Mark an object as a backward-compatibility shim and, where applicable,
    wrap it so that first external use emits a ``DeprecationWarning``.

    The emitted message always names the symbol (``module.qualname``) and
    explains that it is retained only for backward compatibility.

    Behavior by target type:

    * **Class** — registered only; the object is returned unchanged. Paired
      with
      :func:`~snowflake.connector._internal.backward_compatibility.install_backward_compatibility_getattr`,
      the warning fires on first attribute access (e.g.
      ``from ...errors import HttpError`` or ``errors.HttpError``). Classes
      are never wrapped because "use" of a class takes too many shapes
      (``isinstance``, ``raise``, subclassing).

    * **Function / method** — wrapped so the first *external* call emits a
      warning. Calls originating from inside ``snowflake.connector.*`` are
      silent, so the driver's own internal uses don't consume the one-shot
      warning slot before the customer ever sees it.

    * **Descriptor (``property``, ``staticmethod``, ``classmethod``, …)** —
      registered only; the descriptor is returned unchanged. To warn on
      descriptor access, apply ``@backward_compatibility`` to the *raw
      function* (i.e. *below* ``@property`` / ``@prop.setter``) so the
      call-time wrapper is installed before the descriptor is built on top.
    """
    return apply_backward_compatibility(obj)


_TRACKING = ContextVar("_api_tracking", default=True)


def api_telemetry(func: F) -> F:
    """Send api_usage telemetry on the first public-method entry.

    When the decorated method is called and tracking is enabled (the default),
    record the call via :pymeth:`TelemetryClient.send_api_usage` and then
    *disable* tracking for the duration of the method body.  Any nested
    decorated calls (e.g. ``commit`` -> ``cursor`` -> ``execute``) are
    therefore suppressed automatically, ensuring only the outermost
    user-initiated call is recorded.

    For generator functions, tracking stays suppressed for the entire lifetime
    of the generator (including iteration), so that nested decorated calls
    during ``yield`` are also suppressed.

    The ``api_method`` string is derived at runtime as
    ``"{ClassName}.{method_name}"``.
    """
    _is_generator = inspect.isgeneratorfunction(func)

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

            if _is_generator:
                return _suppress_tracking_generator(func(self, *args, **kwargs))

            token = _TRACKING.set(False)
            try:
                return func(self, *args, **kwargs)
            finally:
                _TRACKING.reset(token)
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _suppress_tracking_generator(gen: Any) -> Any:
    """Wrap a generator so _TRACKING is False during each iteration step."""
    token = _TRACKING.set(False)
    try:
        yield from gen
    finally:
        _TRACKING.reset(token)
