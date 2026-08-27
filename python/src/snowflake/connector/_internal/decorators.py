"""Marker, annotation, and precondition wrapper decorators for internal APIs.

The runtime machinery for ``@backward_compatibility`` (call-time wrapper,
module ``__getattr__`` installer, dedup state) lives in
:mod:`._internal.backward_compatibility`; this module is intentionally kept
to just the decorator façade.
"""

from __future__ import annotations

import asyncio
import functools
import inspect
import types

from collections.abc import AsyncGenerator, Awaitable, Callable, Coroutine, Generator
from contextvars import ContextVar
from typing import Any, Generic, ParamSpec, Protocol, TypeVar, cast

from .backward_compatibility import apply_backward_compatibility
from .logging import get_logger


logger = get_logger(__name__)

F = TypeVar("F", bound=Callable[..., Any])
P = ParamSpec("P")

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


def snowpark_compat(func: F) -> F:
    """No-op marker: method/property added only for Snowpark's private-API surface (like :func:`pep249`)."""
    return func


class _AsyncContextManagerLike(Protocol):
    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> Any: ...


R = TypeVar("R", bound=_AsyncContextManagerLike)


class _AwaitableContextManager(Generic[R]):
    """Wrapper returned by :func:`awaitable_context_manager` decorated functions.

    Supports both ``await fn(...)`` and ``async with fn(...) as result:``.
    The ``__aexit__`` delegates to the returned object's ``__aexit__``, so
    the wrapped coroutine must return an async context manager.
    """

    def __init__(self, coro: Coroutine[Any, Any, R]) -> None:
        self._coro = coro
        self._obj: R | None = None

    def __await__(self) -> Generator[Any, None, R]:
        return self._coro.__await__()

    async def __aenter__(self) -> R:
        self._obj = await self._coro
        return self._obj

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> Any:
        if self._obj is not None:
            return await self._obj.__aexit__(exc_type, exc_val, exc_tb)


def awaitable_context_manager(
    func: Callable[P, Coroutine[Any, Any, R]],
) -> Callable[P, _AwaitableContextManager[R]]:
    """Make an ``async def`` factory support both ``await`` and ``async with``.

    The decorated function must return an object that implements
    ``__aenter__`` / ``__aexit__`` (i.e. an async context manager).

    Usage::

        @awaitable_context_manager
        async def connect(**kwargs):
            conn = Connection(**kwargs)
            await conn.open()
            return conn


        # Both patterns work:
        conn = await connect(...)
        async with connect(...) as conn:
            ...
    """

    @functools.wraps(func)
    def wrapper(*args: P.args, **kwargs: P.kwargs) -> _AwaitableContextManager[R]:
        return _AwaitableContextManager(func(*args, **kwargs))

    return wrapper


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

    * **Plain value** (e.g. a module-level string constant) — registered
      only; the value is returned unchanged. Like classes, it warns on first
      attribute access once paired with
      :func:`~snowflake.connector._internal.backward_compatibility.install_backward_compatibility_getattr`,
      since there is no "call" to intercept.
    """
    return apply_backward_compatibility(obj)


# ------------------------------------------------------------------
# Precondition wrappers (connection/cursor @requires_open, etc.)
# ------------------------------------------------------------------


def wrap_method_with_sync_pre(
    func: F,
    *,
    pre: Callable[[Any], None],
) -> F:
    """Wrap *func* with a sync pre-call hook (used for both sync and async methods)."""
    if inspect.isasyncgenfunction(func):

        @functools.wraps(func)
        async def gen_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
            pre(self)
            async for value in func(self, *args, **kwargs):
                yield value

        return cast(F, gen_wrapper)

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


def wrap_method_with_awaitable_pre(
    func: F,
    *,
    sync_pre: Callable[[Any], None],
    async_pre: Callable[[Any], Awaitable[None]],
) -> F:
    """Wrap *func* with sync/async pre-call hooks when the pre itself may await."""
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


_TRACKING: ContextVar[bool] = ContextVar("_api_tracking", default=True)


def _telemetry_client_if_enabled(self: Any) -> Any:
    """Return the telemetry client for *self*, or ``None`` if there is none to send to
    or ``Connection.telemetry_enabled`` is currently ``False``.

    Deliberately re-implements the ``telemetry_enabled`` AND rather than reading
    the public property: that property is ``@api_telemetry``-decorated, and
    calling it from here (inside telemetry dispatch) would recurse into sending
    telemetry about itself.
    """
    from snowflake.connector.aio.connection._connection import Connection as AsyncConnection
    from snowflake.connector.aio.cursor._base import SnowflakeCursorBase as AsyncSnowflakeCursorBase
    from snowflake.connector.connection import Connection
    from snowflake.connector.cursor._base import SnowflakeCursorBase

    if isinstance(self, (Connection, AsyncConnection)):
        connection = self
    elif isinstance(self, (SnowflakeCursorBase, AsyncSnowflakeCursorBase)):
        connection = self._connection
    else:
        raise TypeError(f"Unexpected telemetry target: {type(self)!r}")

    if not (connection._client_param_telemetry_enabled and connection._server_param_telemetry_enabled()):
        return None
    return connection._telemetry_client


def _passed_argument_names(
    sig: inspect.Signature,
    self: Any,
    args: tuple[Any, ...],
    kwargs: dict[str, Any],
) -> list[str]:
    """Return the names of the arguments the caller explicitly passed.

    Signature only — never argument values. Parameters the caller left at
    their default are omitted, because :attr:`BoundArguments.arguments` only
    contains arguments that were actually supplied (we deliberately do not
    call ``apply_defaults()``). ``**kwargs`` entries are expanded to the
    keyword names the caller used, since the var-keyword parameter name
    itself (e.g. ``"kwargs"``) carries no signal. The first positional
    (connection/cursor instance) is dropped.

    Defensive by design: any binding failure yields ``[]`` so telemetry can
    never break the wrapped call.
    """
    try:
        bound = sig.bind(self, *args, **kwargs)
    except TypeError:
        return []
    names: list[str] = []
    for index, (name, bound_arg) in enumerate(bound.arguments.items()):
        if index == 0:
            continue  # first positional (connection/cursor), always bound first by the wrapper
        param = sig.parameters[name]
        if param.kind is inspect.Parameter.VAR_KEYWORD:
            names.extend(bound_arg.keys())
        else:
            names.append(name)
    return names


def _schedule_async_telemetry(coro: Any) -> None:
    """Fire-and-forget async telemetry when a sync decorated method runs under a loop."""
    try:
        asyncio.get_running_loop().create_task(coro)
    except RuntimeError:
        coro.close()
        logger.debug("Skipped async telemetry with no running event loop", exc_info=True)


def _send_api_usage(self: Any, func: Callable[..., Any], passed_arguments: list[str]) -> None:
    """Send the ``{ClassName}.{method_name}`` API-usage telemetry for *self*."""
    try:
        from snowflake.connector._common.telemetry import AsyncTelemetryClient

        api_name = f"{type(self).__name__}.{func.__name__}"
        client = _telemetry_client_if_enabled(self)
        if client is None:
            return
        if isinstance(client, AsyncTelemetryClient):
            _schedule_async_telemetry(client.send_api_usage(api_name, passed_arguments))
        else:
            client.send_api_usage(api_name, passed_arguments)
    except Exception:
        logger.debug("Failed to send api_usage telemetry", exc_info=True)


async def _send_api_usage_async(self: Any, func: Callable[..., Any], passed_arguments: list[str]) -> None:
    """Async counterpart of :func:`_send_api_usage`."""
    try:
        from snowflake.connector._common.telemetry import AsyncTelemetryClient

        api_name = f"{type(self).__name__}.{func.__name__}"
        client = _telemetry_client_if_enabled(self)
        if client is None:
            return
        if isinstance(client, AsyncTelemetryClient):
            await client.send_api_usage(api_name, passed_arguments)
        else:
            client.send_api_usage(api_name, passed_arguments)
    except Exception:
        logger.debug("Failed to send api_usage telemetry", exc_info=True)


def api_telemetry(func: F) -> F:
    """Record ``{ClassName}.{method_name}`` telemetry for the outermost call.

    Suppresses ``_TRACKING`` for the method body so nested decorated calls
    are not recorded.  Generator results are wrapped to suppress only during
    each iteration step (see :func:`_suppress_tracking_generator`).

    Supports synchronous functions, coroutine functions, and async generator
    functions — the suppression always spans the actual execution of the
    wrapped callable (the awaited body or each iteration step), so async
    methods record telemetry exactly like their sync counterparts.

    Alongside the method name, the names of the arguments the caller
    explicitly passed are recorded (see :func:`_passed_argument_names`) —
    names only, never values, and parameters left at their default are
    omitted.

    Free functions (first parameter name is neither ``self`` nor ``cls``) are
    wrapped with a signature-transparent ``(*args, **kwargs)`` wrapper.
    Telemetry dispatch is skipped for free functions because there is no
    ``self`` from which to retrieve the telemetry client; ``_TRACKING``
    suppression still applies so nested decorated calls are not double-counted.
    """
    sig = inspect.signature(func)
    params = list(sig.parameters)
    _is_method = bool(params) and params[0] in ("self", "cls")

    if inspect.iscoroutinefunction(func):
        if _is_method:

            @functools.wraps(func)
            async def async_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
                if _TRACKING.get():
                    await _send_api_usage_async(self, func, _passed_argument_names(sig, self, args, kwargs))
                    _TRACKING.set(False)
                    try:
                        return await func(self, *args, **kwargs)
                    finally:
                        _TRACKING.set(True)
                return await func(self, *args, **kwargs)

            return cast(F, async_wrapper)

        @functools.wraps(func)
        async def async_free_wrapper(*args: Any, **kwargs: Any) -> Any:
            if _TRACKING.get():
                _TRACKING.set(False)
                try:
                    return await func(*args, **kwargs)
                finally:
                    _TRACKING.set(True)
            return await func(*args, **kwargs)

        return cast(F, async_free_wrapper)

    if inspect.isasyncgenfunction(func):
        if _is_method:

            @functools.wraps(func)
            async def async_gen_wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
                if _TRACKING.get():
                    await _send_api_usage_async(self, func, _passed_argument_names(sig, self, args, kwargs))
                    async for value in _suppress_tracking_async_generator(func(self, *args, **kwargs)):
                        yield value
                else:
                    async for value in func(self, *args, **kwargs):
                        yield value

            return cast(F, async_gen_wrapper)

        @functools.wraps(func)
        async def async_gen_free_wrapper(*args: Any, **kwargs: Any) -> Any:
            if _TRACKING.get():
                async for value in _suppress_tracking_async_generator(func(*args, **kwargs)):
                    yield value
            else:
                async for value in func(*args, **kwargs):
                    yield value

        return cast(F, async_gen_free_wrapper)

    if _is_method:

        @functools.wraps(func)
        def wrapper(self: Any, *args: Any, **kwargs: Any) -> Any:
            if _TRACKING.get():
                if func.__name__ == "__init__":
                    # Compute argument names before calling (kwargs are not consumed by
                    # the call), but send telemetry post-call so _telemetry_client is ready.
                    passed_args = _passed_argument_names(sig, self, args, kwargs)
                    _TRACKING.set(False)
                    try:
                        result = func(self, *args, **kwargs)
                    finally:
                        _TRACKING.set(True)
                    _send_api_usage(self, func, passed_args)
                    return result

                _send_api_usage(self, func, _passed_argument_names(sig, self, args, kwargs))

                _TRACKING.set(False)
                try:
                    result = func(self, *args, **kwargs)
                finally:
                    _TRACKING.set(True)

                if isinstance(result, types.GeneratorType):
                    return _suppress_tracking_generator(result)

                return result
            return func(self, *args, **kwargs)

        return cast(F, wrapper)

    @functools.wraps(func)
    def free_wrapper(*args: Any, **kwargs: Any) -> Any:
        if _TRACKING.get():
            _TRACKING.set(False)
            try:
                result = func(*args, **kwargs)
            finally:
                _TRACKING.set(True)

            if isinstance(result, types.GeneratorType):
                return _suppress_tracking_generator(result)

            return result
        return func(*args, **kwargs)

    return cast(F, free_wrapper)


def _suppress_tracking_generator(
    gen: Generator[Any, Any, Any],
) -> Generator[Any, Any, Any]:
    """Suppress ``_TRACKING`` during each iteration step, restore between yields.

    Safe against never-started generators: the wrapper body only runs when
    iterated, and ``api_telemetry`` resets ``_TRACKING`` before creating it.
    """
    _TRACKING.set(False)
    try:
        for value in gen:
            _TRACKING.set(True)
            try:
                yield value
            except GeneratorExit:
                gen.close()
                return
            _TRACKING.set(False)
    finally:
        _TRACKING.set(True)


async def _suppress_tracking_async_generator(
    agen: AsyncGenerator[Any, Any],
) -> AsyncGenerator[Any, Any]:
    """Async counterpart of :func:`_suppress_tracking_generator`.

    Suppress ``_TRACKING`` during each iteration step, restore between yields.
    """
    _TRACKING.set(False)
    try:
        async for value in agen:
            _TRACKING.set(True)
            try:
                yield value
            except GeneratorExit:
                await agen.aclose()
                return
            _TRACKING.set(False)
    finally:
        _TRACKING.set(True)
