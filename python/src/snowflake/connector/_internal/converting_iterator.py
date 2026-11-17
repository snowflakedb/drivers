"""Row iterator wrappers that apply the per-column callables of an ``ArrowConverterContext``."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING, Any


if TYPE_CHECKING:
    from .arrow import ArrowRowIterator, AsyncArrowRowIterator
    from .arrow_context import ArrowConverterContext
    from .cursor.result_metadata import ResultMetadata

Converters = list[Callable[[Any], Any] | None]


def _convert_row(row: Any, converters: Converters) -> Any:
    if isinstance(row, dict):
        return {
            key: value if conv is None or value is None else conv(value)
            for (key, value), conv in zip(row.items(), converters, strict=True)
        }
    return tuple(
        value if conv is None or value is None else conv(value) for value, conv in zip(row, converters, strict=True)
    )


class ConvertingRowIterator:
    def __init__(self, iterator: ArrowRowIterator, converters: Converters) -> None:
        self._iterator = iterator
        self._converters = converters

    def __iter__(self) -> ConvertingRowIterator:
        return self

    def __next__(self) -> Any:
        return _convert_row(next(self._iterator), self._converters)

    def fetch_many(self, size: int) -> list[Any]:
        return [_convert_row(row, self._converters) for row in self._iterator.fetch_many(size)]

    def fetch_all(self) -> list[Any]:
        return [_convert_row(row, self._converters) for row in self._iterator.fetch_all()]


class AsyncConvertingRowIterator:
    def __init__(self, iterator: AsyncArrowRowIterator, converters: Converters) -> None:
        self._iterator = iterator
        self._converters = converters

    def __aiter__(self) -> AsyncConvertingRowIterator:
        return self

    async def __anext__(self) -> Any:
        return _convert_row(await self._iterator.__anext__(), self._converters)

    async def fetch_next(self, default: object = None) -> Any:
        sentinel = object()
        row = await self._iterator.fetch_next(sentinel)
        return default if row is sentinel else _convert_row(row, self._converters)

    async def fetch_many(self, size: int) -> list[Any]:
        return [_convert_row(row, self._converters) for row in await self._iterator.fetch_many(size)]

    async def fetch_all(self) -> list[Any]:
        return [_convert_row(row, self._converters) for row in await self._iterator.fetch_all()]


def wrap_row_iterator(
    iterator: ArrowRowIterator, context: ArrowConverterContext, description: Sequence[ResultMetadata] | None
) -> ArrowRowIterator:
    """Return ``iterator`` itself unless ``context`` defines a converter for some column."""
    converters = context._column_converters(description) if description else None
    return ConvertingRowIterator(iterator, converters) if converters else iterator


def wrap_async_row_iterator(
    iterator: AsyncArrowRowIterator, context: ArrowConverterContext, description: Sequence[ResultMetadata] | None
) -> AsyncArrowRowIterator:
    """Return ``iterator`` itself unless ``context`` defines a converter for some column."""
    converters = context._column_converters(description) if description else None
    return AsyncConvertingRowIterator(iterator, converters) if converters else iterator
