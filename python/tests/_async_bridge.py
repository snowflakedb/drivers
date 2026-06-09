"""Test-only blocking facade that runs the async cursor through the sync API.

The async cursor (``snowflake.connector._async.cursor``) is a coroutine-based
duplicate of the sync cursor. To exercise it with the *existing* synchronous
``test_cursor.py`` suite — without rewriting any test — we wrap an async cursor
in :class:`BlockingCursor`, which drives every coroutine / async-generator to
completion on a dedicated background event loop and exposes the results through
the ordinary blocking PEP 249 surface.

A :class:`BlockingConnection` proxies a real (sync) connection but hands out
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

from snowflake.connector._async.cursor import AsyncDictCursor as AsyncDictCursor
from snowflake.connector._async.cursor import AsyncSnowflakeCursor as AsyncSnowflakeCursor
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


class BlockingCursor:
    """Synchronous facade over a single async cursor instance.

    Method calls that return coroutines are awaited to completion; methods that
    return async generators are surfaced as ordinary (sync) generators. Property
    reads, data attributes, and plain sync methods pass straight through. When an
    async method returns the underlying async cursor (``return self``), this
    facade substitutes *itself* so identity assertions like ``cur.execute(...) is
    cur`` keep holding.
    """

    def __init__(self, async_cursor: Any, loop: _LoopRunner) -> None:
        object.__setattr__(self, "_async_cursor", async_cursor)
        object.__setattr__(self, "_loop", loop)

    def __getattr__(self, name: str) -> Any:
        if name in ("_async_cursor", "_loop"):
            raise AttributeError(name)
        async_cursor = object.__getattribute__(self, "_async_cursor")

        # Properties / data attributes resolve to their live value untouched —
        # only class-level methods get the blocking treatment. This keeps
        # callable-valued properties (e.g. ``errorhandler``) intact.
        static = inspect.getattr_static(type(async_cursor), name, _MISSING)
        if isinstance(static, property) or static is _MISSING or not callable(static):
            return getattr(async_cursor, name)

        bound = getattr(async_cursor, name)

        @functools.wraps(bound)
        def blocking(*args: Any, **kwargs: Any) -> Any:
            result = bound(*args, **kwargs)
            if inspect.iscoroutine(result):
                result = self._loop.run(result)
            elif inspect.isasyncgen(result):
                return self._loop.iterate(result)
            return self if result is async_cursor else result

        return blocking

    def __setattr__(self, name: str, value: Any) -> None:
        if name in ("_async_cursor", "_loop"):
            object.__setattr__(self, name, value)
        else:
            setattr(self._async_cursor, name, value)

    # -- iterator protocol --------------------------------------------------

    def __iter__(self) -> BlockingCursor:
        return self

    def __next__(self) -> Any:
        try:
            return self._loop.run(self._async_cursor.__anext__())
        except StopAsyncIteration:
            raise StopIteration from None

    # -- context manager ----------------------------------------------------

    def __enter__(self) -> BlockingCursor:
        self._loop.run(self._async_cursor.__aenter__())
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> Any:
        return self._loop.run(self._async_cursor.__aexit__(exc_type, exc_val, exc_tb))


class BlockingConnection:
    """Proxy around a real sync connection that vends :class:`BlockingCursor`.

    Everything except :meth:`cursor` delegates to the wrapped connection, so the
    connection itself stays on the (out-of-scope, still-synchronous) sync path
    while cursors created from it run the async implementation.
    """

    def __init__(self, connection: Any, loop: _LoopRunner) -> None:
        object.__setattr__(self, "_connection", connection)
        object.__setattr__(self, "_loop", loop)

    def cursor(self, cursor_class: type = SyncSnowflakeCursor) -> BlockingCursor:
        async_cls = _ASYNC_CURSOR_FOR.get(cursor_class)
        if async_cls is None:
            raise ValueError(f"No async cursor counterpart registered for {cursor_class!r}")
        return BlockingCursor(async_cls(self._connection), self._loop)

    def __getattr__(self, name: str) -> Any:
        if name in ("_connection", "_loop"):
            raise AttributeError(name)
        return getattr(object.__getattribute__(self, "_connection"), name)

    def __setattr__(self, name: str, value: Any) -> None:
        if name in ("_connection", "_loop"):
            object.__setattr__(self, name, value)
        else:
            setattr(self._connection, name, value)

    def __enter__(self) -> BlockingConnection:
        self._connection.__enter__()
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> Any:
        return self._connection.__exit__(exc_type, exc_val, exc_tb)


def maybe_blocking_async(connection: Any, cursor_backend: str) -> Any:
    """Return *connection* unchanged for the sync backend, or wrapped for async."""
    if cursor_backend == "async":
        return BlockingConnection(connection, _LoopRunner.instance())
    return connection
