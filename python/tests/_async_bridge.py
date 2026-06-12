"""Test-only blocking facade that runs the async cursor through the sync API.

The async cursor (``snowflake.connector.aio.cursor``) is a coroutine-based
duplicate of the sync cursor. To exercise it with the *existing* synchronous
``test_cursor.py`` suite — without rewriting any test — we wrap an async cursor
in :class:`BlockingCursor`, which drives every coroutine / async-generator to
completion on a dedicated background event loop and exposes the results through
the ordinary blocking PEP 249 surface.

:class:`BlockingConnection` opens an :class:`~snowflake.connector.aio.Connection`
(via :func:`~snowflake.connector.aio.connect`) and hands out
:class:`BlockingCursor` instances from :meth:`cursor`, so tests that call
``connection.cursor()`` directly transparently get the async implementation.

This lives under ``tests/`` and must never be imported by production code.
"""

from __future__ import annotations

import asyncio
import functools
import inspect
import threading

from typing import Any

from snowflake.connector.aio import connect
from snowflake.connector.aio.cursor import DictCursor as AsyncDictCursor
from snowflake.connector.aio.cursor import SnowflakeCursor as AsyncSnowflakeCursor
from snowflake.connector.cursor import DictCursor as SyncDictCursor
from snowflake.connector.cursor import SnowflakeCursor as SyncSnowflakeCursor


# Map the sync cursor class a test asks for to its async counterpart.
_ASYNC_CURSOR_FOR: dict[type, type] = {
    SyncSnowflakeCursor: AsyncSnowflakeCursor,
    SyncDictCursor: AsyncDictCursor,
}

_MISSING = object()


class _LoopRunner:
    """Owns a background event loop and runs coroutines on it synchronously.

    A single loop runs forever in a daemon thread for the whole test process.
    ``run`` submits a coroutine and blocks the calling (test) thread until it
    completes — the standard "sync facade over async" pattern. ``iterate``
    drives an async generator one item at a time the same way.
    """

    _instance: _LoopRunner | None = None
    _instance_lock = threading.Lock()

    def __init__(self) -> None:
        self._loop = asyncio.new_event_loop()
        self._thread = threading.Thread(target=self._loop.run_forever, name="blocking-async-loop", daemon=True)
        self._thread.start()

    @classmethod
    def instance(cls) -> _LoopRunner:
        if cls._instance is None:
            with cls._instance_lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    def run(self, coro: Any) -> Any:
        return asyncio.run_coroutine_threadsafe(coro, self._loop).result()

    def iterate(self, agen: Any) -> Any:
        while True:
            try:
                item = self.run(agen.__anext__())
            except StopAsyncIteration:
                return
            yield item


def _blocking_method(facade: Any, async_obj: Any, loop: _LoopRunner, bound: Any) -> Any:
    @functools.wraps(bound)
    def blocking(*args: Any, **kwargs: Any) -> Any:
        result = bound(*args, **kwargs)
        if inspect.iscoroutine(result):
            result = loop.run(result)
        elif inspect.isasyncgen(result):
            return loop.iterate(result)
        return facade if result is async_obj else result

    return blocking


class _BlockingAsyncFacade:
    """Synchronous facade over an async connection or cursor instance."""

    def __init__(self, async_obj: Any, loop: _LoopRunner) -> None:
        object.__setattr__(self, "_async_obj", async_obj)
        object.__setattr__(self, "_loop", loop)

    def __getattr__(self, name: str) -> Any:
        if name in ("_async_obj", "_loop"):
            raise AttributeError(name)
        async_obj = object.__getattribute__(self, "_async_obj")

        static = inspect.getattr_static(type(async_obj), name, _MISSING)
        if isinstance(static, property) or static is _MISSING or not callable(static):
            return getattr(async_obj, name)

        bound = getattr(async_obj, name)
        return _blocking_method(self, async_obj, object.__getattribute__(self, "_loop"), bound)

    def __setattr__(self, name: str, value: Any) -> None:
        if name in ("_async_obj", "_loop"):
            object.__setattr__(self, name, value)
        else:
            setattr(self._async_obj, name, value)


class BlockingCursor(_BlockingAsyncFacade):
    """Synchronous facade over a single async cursor instance.

    When an async method returns the underlying async cursor (``return self``),
    this facade substitutes *itself* so identity assertions like ``cur.execute(...) is
    cur`` keep holding.
    """

    def __iter__(self) -> BlockingCursor:
        return self

    def __next__(self) -> Any:
        loop = object.__getattribute__(self, "_loop")
        async_cursor = object.__getattribute__(self, "_async_obj")
        try:
            return loop.run(async_cursor.__anext__())
        except StopAsyncIteration:
            raise StopIteration from None

    def __enter__(self) -> BlockingCursor:
        loop = object.__getattribute__(self, "_loop")
        async_cursor = object.__getattribute__(self, "_async_obj")
        loop.run(async_cursor.__aenter__())
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> Any:
        loop = object.__getattribute__(self, "_loop")
        async_cursor = object.__getattribute__(self, "_async_obj")
        return loop.run(async_cursor.__aexit__(exc_type, exc_val, exc_tb))


class BlockingConnection(_BlockingAsyncFacade):
    """Proxy around an aio :class:`~snowflake.connector.aio.Connection` that vends :class:`BlockingCursor`."""

    def _async_cursor_class(self, cursor_class: type) -> type:
        async_cls = _ASYNC_CURSOR_FOR.get(cursor_class)
        if async_cls is None:
            raise ValueError(f"No async cursor counterpart registered for {cursor_class!r}")
        return async_cls

    def cursor(self, cursor_class: type = SyncSnowflakeCursor) -> BlockingCursor:
        async_connection = object.__getattribute__(self, "_async_obj")
        loop = object.__getattribute__(self, "_loop")
        async_cur = loop.run(async_connection.cursor(cursor_class=self._async_cursor_class(cursor_class)))
        return BlockingCursor(async_cur, loop)

    def execute_string(
        self,
        sql_text: str,
        remove_comments: bool = False,
        return_cursors: bool = True,
        cursor_class: type = SyncSnowflakeCursor,
        **kwargs: Any,
    ) -> Any:
        async_connection = object.__getattribute__(self, "_async_obj")
        loop = object.__getattribute__(self, "_loop")
        result = loop.run(
            async_connection.execute_string(
                sql_text,
                remove_comments=remove_comments,
                return_cursors=return_cursors,
                cursor_class=self._async_cursor_class(cursor_class),
                **kwargs,
            )
        )
        if not return_cursors:
            return result
        return [BlockingCursor(cur, loop) for cur in result]

    def execute_stream(
        self,
        stream: Any,
        remove_comments: bool = False,
        cursor_class: type = SyncSnowflakeCursor,
        **kwargs: Any,
    ) -> Any:
        async_connection = object.__getattribute__(self, "_async_obj")
        loop = object.__getattribute__(self, "_loop")

        def _stream() -> Any:
            agen = async_connection.execute_stream(
                stream,
                remove_comments=remove_comments,
                cursor_class=self._async_cursor_class(cursor_class),
                **kwargs,
            )
            yield from (BlockingCursor(cur, loop) for cur in loop.iterate(agen))

        return _stream()

    def __enter__(self) -> BlockingConnection:
        loop = object.__getattribute__(self, "_loop")
        async_connection = object.__getattribute__(self, "_async_obj")
        loop.run(async_connection.__aenter__())
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> Any:
        loop = object.__getattribute__(self, "_loop")
        async_connection = object.__getattribute__(self, "_async_obj")
        return loop.run(async_connection.__aexit__(exc_type, exc_val, exc_tb))


def maybe_blocking_async_connection(connection: Any) -> BlockingConnection:
    """Open an aio :class:`~snowflake.connector.aio.Connection` and expose it through :class:`BlockingConnection`."""
    loop = _LoopRunner.instance()

    async def _open() -> Any:
        # connect_async is @awaitable_context_manager — returns _AwaitableContextManager,
        # not a bare coroutine, so it must be awaited inside an async helper.
        return await connect(config=connection.config)

    async_connection = loop.run(_open())
    connection.close()
    return BlockingConnection(async_connection, loop)


def maybe_blocking_async(connection: Any, cursor_backend: str) -> Any:
    """Return *connection* unchanged for the sync backend, or wrapped for async."""
    if cursor_backend != "async":
        return connection
    return maybe_blocking_async_connection(connection)
