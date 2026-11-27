"""Run blocking native calls without stalling a gevent hub.

gevent yields at monkey-patched blocking operations. A wait inside the Rust
core is not one, so a gunicorn gevent worker serves no other request until the
call returns.

When ``socket`` is gevent-patched and the caller is a greenlet on the hub
thread, the call runs on a driver-owned gevent ``ThreadPool`` and the calling
greenlet yields until it returns. In every other case the call runs inline.

Each in-flight call holds one pool thread for its whole duration. The pool has
``DEFAULT_THREADPOOL_SIZE`` threads; ``SNOWFLAKE_GEVENT_THREADPOOL_SIZE``
overrides that. Calls beyond the limit queue until a thread frees up. The pool
is separate from ``hub.threadpool``, which gevent's default resolver also uses.

Cancelling a greenlet (``kill()``, ``gevent.Timeout``, gunicorn's graceful
shutdown) ends the wait but not the native call, which keeps running on its
pool thread and cannot be cancelled. The result of an abandoned ``call_proto``
is dropped and its query keeps running on the server. An iterator whose fetch
was abandoned refuses further calls with ``OperationalError``, because the
abandoned fetch still owns the native iterator and its rows are lost.
"""

from __future__ import annotations

import contextvars
import functools
import logging
import os
import sys
import weakref

from collections import deque
from collections.abc import Callable
from typing import TYPE_CHECKING, Any, TypeVar

from snowflake.connector.errors import OperationalError


if TYPE_CHECKING:
    from .arrow import ArrowRowIterator, ArrowTableIterator

logger = logging.getLogger(__name__)

DEFAULT_THREADPOOL_SIZE = 32
THREADPOOL_SIZE_ENV_VAR = "SNOWFLAKE_GEVENT_THREADPOOL_SIZE"
_ROW_BUFFER_SIZE = 1024

_T = TypeVar("_T")
_END = object()
_pools: weakref.WeakKeyDictionary[Any, Any] = weakref.WeakKeyDictionary()


@functools.cache
def _original_get_ident(monkey: Any) -> Callable[[], int]:
    ident: Callable[[], int] = monkey.get_original("_thread", "get_ident")
    return ident


def _gevent_patched() -> bool:
    monkey = sys.modules.get("gevent.monkey")
    return monkey is not None and bool(monkey.is_module_patched("socket"))


def _existing_hub() -> Any | None:
    """Return this thread's hub without creating one, which ``gevent.get_hub()`` would do."""
    try:
        from gevent._hub_local import get_hub_if_exists
    except ImportError:
        from gevent import get_hub as get_hub_if_exists
    return get_hub_if_exists()


def _offload_hub() -> Any | None:
    """Return the hub to offload to, or ``None`` when the call must run inline."""
    if not _gevent_patched():
        return None

    from gevent import getcurrent

    hub = _existing_hub()
    if hub is None or getcurrent() is hub:
        return None
    monkey = sys.modules["gevent.monkey"]
    if hub.thread_ident != _original_get_ident(monkey)():
        return None
    return hub


def _threadpool_size() -> int:
    raw = os.environ.get(THREADPOOL_SIZE_ENV_VAR)
    if raw is None:
        return DEFAULT_THREADPOOL_SIZE
    try:
        size = int(raw)
    except ValueError:
        size = 0
    if size < 1:
        logger.warning(
            "Ignoring %s=%r: expected a positive integer. Using %d threads.",
            THREADPOOL_SIZE_ENV_VAR,
            raw,
            DEFAULT_THREADPOOL_SIZE,
        )
        return DEFAULT_THREADPOOL_SIZE
    return size


def _threadpool_for(hub: Any) -> Any:
    pool = _pools.get(hub)
    if pool is None:
        from gevent.threadpool import ThreadPool

        size = _threadpool_size()
        pool = _pools[hub] = ThreadPool(size, hub=hub)
        logger.info(
            "gevent detected: blocking native calls run on a %d-thread pool. Set %s to change the size.",
            size,
            THREADPOOL_SIZE_ENV_VAR,
        )
    return pool


def _run_on_pool(hub: Any, fn: Callable[..., _T], args: tuple[Any, ...], kwargs: dict[str, Any]) -> _T:
    pool = _threadpool_for(hub)
    if len(pool) >= pool.maxsize:
        logger.debug(
            "gevent pool saturated (%d pending calls, %d threads); this call waits for a free thread. "
            "Set %s to raise the limit.",
            len(pool),
            pool.maxsize,
            THREADPOOL_SIZE_ENV_VAR,
        )
    ctx = contextvars.copy_context()
    result: _T = pool.apply(ctx.run, (fn, *args), kwargs)
    return result


def run_native_blocking(fn: Callable[..., _T], /, *args: Any, **kwargs: Any) -> _T:
    """Call ``fn`` on the gevent thread pool when running it inline would stall the hub."""
    hub = _offload_hub()
    if hub is None:
        return fn(*args, **kwargs)
    return _run_on_pool(hub, fn, args, kwargs)


class _OffloadedIterator:
    __slots__ = ("_interrupted",)

    def __init__(self) -> None:
        self._interrupted = False

    def _offload(self, fn: Callable[..., _T], *args: Any) -> _T:
        if self._interrupted:
            raise OperationalError(
                msg=(
                    "A previous fetch on this result was interrupted (greenlet killed or gevent.Timeout) "
                    "while the native call was still running, so its rows are lost. "
                    "Re-execute the query to fetch the result again."
                )
            )
        hub = _offload_hub()
        if hub is None:
            return fn(*args)
        try:
            return _run_on_pool(hub, fn, args, {})
        except Exception:
            raise
        except BaseException:
            self._interrupted = True
            raise


class OffloadedRowIterator(_OffloadedIterator):
    """Row iterator that loads rows in batches on the gevent thread pool.

    Only the batch load waits inside the core. Rows are served from a local
    buffer so that a per-row ``next()`` does not pay a thread-pool round trip.
    """

    __slots__ = ("_buffer", "_iterator")

    def __init__(self, iterator: ArrowRowIterator) -> None:
        super().__init__()
        self._iterator = iterator
        self._buffer: deque[Any] = deque()

    def __iter__(self) -> OffloadedRowIterator:
        return self

    def __next__(self) -> Any:
        if self._buffer:
            return self._buffer.popleft()
        self._buffer.extend(self._offload(self._iterator.fetch_many, _ROW_BUFFER_SIZE))
        if not self._buffer:
            raise StopIteration
        return self._buffer.popleft()

    def fetch_many(self, size: int) -> list[Any]:
        rows = [self._buffer.popleft() for _ in range(min(size, len(self._buffer)))]
        if len(rows) < size:
            rows.extend(self._offload(self._iterator.fetch_many, size - len(rows)))
        return rows

    def fetch_all(self) -> list[Any]:
        rows = list(self._buffer)
        self._buffer.clear()
        rows.extend(self._offload(self._iterator.fetch_all))
        return rows


class OffloadedTableIterator(_OffloadedIterator):
    """Record batch iterator that loads each batch on the gevent thread pool."""

    __slots__ = ("_iterator",)

    def __init__(self, iterator: ArrowTableIterator) -> None:
        super().__init__()
        self._iterator = iterator

    def __iter__(self) -> OffloadedTableIterator:
        return self

    def __next__(self) -> Any:
        batch = self._offload(next, self._iterator, _END)
        if batch is _END:
            raise StopIteration
        return batch

    def get_converted_schema(self) -> Any:
        return self._iterator.get_converted_schema()


def offload_row_iterator(iterator: ArrowRowIterator) -> ArrowRowIterator:
    """Wrap ``iterator`` when gevent has patched sockets; each call then decides whether to offload."""
    return OffloadedRowIterator(iterator) if _gevent_patched() else iterator


def offload_table_iterator(iterator: ArrowTableIterator) -> ArrowTableIterator:
    """Wrap ``iterator`` when gevent has patched sockets; each call then decides whether to offload."""
    return OffloadedTableIterator(iterator) if _gevent_patched() else iterator
