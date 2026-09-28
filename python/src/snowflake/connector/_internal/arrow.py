"""Shared protocols for Arrow row iterators (Cython nanoarrow and native PyO3)."""

from __future__ import annotations

from collections.abc import AsyncIterator
from typing import Any, Protocol


class ArrowRowIterator(Protocol):
    """Row iterator produced by ``create_row_iterator``.

    Both the Cython ``ArrowStreamIterator`` and the PyO3
    ``sf_core_python.ArrowStreamIterator`` (``native-arrow`` builds) satisfy
    this protocol, so call sites do not branch on the backend.
    """

    def __iter__(self) -> ArrowRowIterator: ...
    def __next__(self) -> Any: ...
    def fetch_many(self, size: int) -> list[Any]: ...
    def fetch_all(self) -> list[Any]: ...


class AsyncArrowRowIterator(Protocol):
    """Async row iterator used by the aio cursor and ResultBatch.

    The ``to_thread`` wrapper over a sync iterator and the driver over the
    PyO3 batch reader (``native-arrow`` builds) both satisfy this protocol.
    """

    def __aiter__(self) -> AsyncIterator[Any]: ...
    async def __anext__(self) -> Any: ...
    async def fetch_next(self, default: object = None) -> Any: ...
    async def fetch_many(self, size: int) -> list[Any]: ...
    async def fetch_all(self) -> list[Any]: ...
