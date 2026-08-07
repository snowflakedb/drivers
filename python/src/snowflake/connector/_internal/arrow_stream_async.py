"""Async adapters over the synchronous nanoarrow iterators.

Why every advance goes through ``asyncio.to_thread``
----------------------------------------------------
The Arrow stream iterators (:class:`ArrowStreamIterator`,
:class:`ArrowStreamTableIterator`) and the helpers in :mod:`arrow_stream_utils`
expose a purely synchronous API: there is no native async entry point. A single
``next()`` does two kinds of work, and both are unfriendly to the event loop:

1. **Thread-blocking I/O at chunk boundaries.** Under the hood the core builds a
   prefetching reader whose ``next()`` calls ``blocking_recv()`` on a channel:
   it parks the *calling OS thread* until a background task has finished
   downloading and parsing the next result chunk. The downloads themselves run
   on the core's own (multi-threaded Tokio) runtime, so they do **not** depend
   on the Python event loop, but the consuming ``next()`` still blocks its
   thread waiting for them.
2. **GIL-bound CPU work.** Decoding a row/batch into Python objects (building
   tuples/dicts, running the per-column converters) holds the GIL the entire
   time. Only the chunk-boundary fetch above releases the GIL; the decode does
   not.

If we called ``next()`` directly inside a coroutine, both of those would run on
the event-loop thread: chunk-boundary advances would block the loop on network
I/O, and every decode would hold the GIL, so no other task could make progress.
This does *not* deadlock (the downloads complete independently on the core
runtime), but it defeats the point of an async API. Offloading to a worker
thread via :func:`asyncio.to_thread` keeps the loop free while a blocking
advance is in flight.

So why offload *every* call rather than just the slow ones? From Python there
is no reliable way to tell a cheap advance (popping an already-buffered row)
from an expensive one (a chunk-boundary ``blocking_recv`` or a heavy decode), so
the uniform, safe choice is to offload them all and accept a thread handoff on
the cheap ones.

Possible modifications (intentionally not done yet)
---------------------------------------------------
The row path pays this hop *per row*: ``__anext__`` / :meth:`fetch_next` offload
a single ``next()`` each, so a large result set incurs one ``to_thread`` hop
(and a round of GIL ping-pong) for every row. ``fetch_many`` / ``fetch_all``
already avoid this by batching many rows into one native call under a single
hop. Row iteration could be made similarly cheap by backing ``__anext__`` with
an internal buffer refilled via :meth:`fetch_many` under one hop, instead of one
hop per row.

Alternatively, if performance tests show the offload is unnecessary for the
common case (e.g. the loop-blocking cost is negligible in practice), the advance
could simply be a plain synchronous ``next()`` call with no ``to_thread`` hop at
all. Left as-is for now; revisit either way if async iteration shows up as a
bottleneck.


This module centralizes those ``to_thread`` hops behind small async wrappers so
the async cursor and async result batch share one implementation (and one
place documenting this rationale) instead of repeating
``await asyncio.to_thread(...)`` at every call site.
"""

from __future__ import annotations

import asyncio

from collections.abc import AsyncIterator, Sequence
from typing import TYPE_CHECKING, Any

from .arrow_stream_utils import collect_arrow_table


if TYPE_CHECKING:
    from pyarrow import Table

    from .arrow_stream_iterator import ArrowStreamIterator, ArrowStreamTableIterator
    from .cursor.result_metadata import ResultMetadata


# Sentinel handed to ``next(iterator, default)`` so a ``StopIteration`` raised
# inside a worker thread never has to cross the ``to_thread`` boundary (where it
# would be wrapped in confusing ways). The caller compares identity against it.
_ITER_DONE = object()


def _next_or_default(iterator: Any, default: object) -> Any:
    """Typed ``next(iterator, default)`` suitable for :func:`asyncio.to_thread`.

    The builtin ``next`` is overloaded, which mypy refuses to pass directly to
    ``to_thread``. This thin wrapper gives it a concrete, checkable signature.
    """
    return next(iterator, default)


class AsyncArrowStreamIterator(AsyncIterator[Any]):
    """Async facade over a synchronous nanoarrow iterator.

    Wraps an :class:`ArrowStreamIterator` (row iterator) or an
    :class:`ArrowStreamTableIterator` (RecordBatch iterator) and exposes async
    iteration plus bulk ``fetch_many`` / ``fetch_all`` helpers. Every advance of
    the underlying iterator runs on a worker thread via :func:`asyncio.to_thread`
    (see module docstring for why).
    """

    def __init__(self, iterator: ArrowStreamIterator | ArrowStreamTableIterator) -> None:
        self._iterator = iterator

    def __aiter__(self) -> AsyncArrowStreamIterator:
        return self

    async def __anext__(self) -> Any:
        item = await asyncio.to_thread(_next_or_default, self._iterator, _ITER_DONE)
        if item is _ITER_DONE:
            raise StopAsyncIteration
        return item

    async def fetch_next(self, default: object = None) -> Any:
        """Return the next item, or *default* when the stream is exhausted.

        Unlike :meth:`__anext__` this never raises ``StopAsyncIteration``; it
        is the async analogue of ``next(it, default)`` and is convenient for
        manual fetch loops (e.g. ``fetchone``).
        """
        item = await asyncio.to_thread(_next_or_default, self._iterator, _ITER_DONE)
        return default if item is _ITER_DONE else item

    async def fetch_many(self, size: int) -> list[Any]:
        """Fetch up to *size* rows in a single threaded C++ call."""
        return await asyncio.to_thread(self._iterator.fetch_many, size)  # type: ignore[union-attr]

    async def fetch_all(self) -> list[Any]:
        """Fetch all remaining rows in a single threaded C++ call."""
        return await asyncio.to_thread(self._iterator.fetch_all)  # type: ignore[union-attr]


async def collect_arrow_table_async(
    table_iterator: ArrowStreamTableIterator,
    columns_metadata: Sequence[ResultMetadata] | None = None,
    force_return_table: bool = False,
) -> Table | None:
    """Async wrapper for :func:`collect_arrow_table`.

    Draining *table_iterator* into a single Arrow table runs every blocking
    ``next()`` plus the final ``Table.from_batches`` on a worker thread.
    """
    return await asyncio.to_thread(
        collect_arrow_table,
        table_iterator,
        columns_metadata,
        force_return_table,
    )


async def to_pandas_async(table: Table, **kwargs: Any) -> Any:
    """Convert an Arrow table to a pandas DataFrame off the event loop.

    ``Table.to_pandas`` is CPU-heavy C/Cython work with no async entry point, so
    it is offloaded for the same reason as the fetch path. ``**kwargs`` are
    forwarded to ``Table.to_pandas`` (e.g. ``split_blocks``, ``self_destruct``).
    """
    return await asyncio.to_thread(table.to_pandas, **kwargs)
