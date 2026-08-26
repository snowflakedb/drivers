"""Shared type for sync Arrow row iterators (Cython nanoarrow and native PyO3)."""

from __future__ import annotations

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
