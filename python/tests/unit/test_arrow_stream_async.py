from __future__ import annotations

import asyncio
import threading

from typing import Any
from unittest.mock import AsyncMock, MagicMock, patch

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.arrow_stream_async import (
    NativeAsyncArrowRowIterator,
    create_async_row_iterator_from_result_set,
    create_async_row_iterator_from_stream_ptr,
)


def test_create_async_row_iterator_from_result_set():
    mock_iterator = MagicMock(name="native_async_iterator")
    mock_class = MagicMock()
    mock_class.from_result_set = AsyncMock(return_value=mock_iterator)
    mock_core = MagicMock()
    mock_core.AsyncArrowStreamIterator = mock_class
    handle = MagicMock()
    handle.id = 7
    handle.magic = 11

    with patch(
        "snowflake.connector._internal.arrow_stream_async.sf_core_python",
        mock_core,
    ):
        result = asyncio.run(
            create_async_row_iterator_from_result_set(
                handle,
                context=ArrowConverterContext(timezone="UTC"),
                use_dict_result=True,
                use_numpy=True,
            )
        )

    assert isinstance(result, NativeAsyncArrowRowIterator)
    mock_class.from_result_set.assert_awaited_once_with(
        7,
        11,
        session_timezone="UTC",
        use_dict_result=True,
        use_numpy=True,
    )


class _FakeBatchReader:
    """Two single-row batches, recording the thread each sync call runs on."""

    def __init__(self, batches: list[list[int]]) -> None:
        self._batches = list(batches)
        self._buffered: list[int] = []
        self._converted: list[int] = []
        self.sync_call_threads: list[int] = []

    async def next_batch(self) -> bool:
        await asyncio.sleep(0)
        if not self._batches:
            return False
        self._buffered = self._batches.pop(0)
        return True

    def take_row(self, default: object = None) -> Any:
        self.sync_call_threads.append(threading.get_ident())
        return self._buffered.pop(0) if self._buffered else default

    def convert(self, count: int | None = None) -> int:
        self.sync_call_threads.append(threading.get_ident())
        take = len(self._buffered) if count is None else min(count, len(self._buffered))
        self._converted.extend(self._buffered[:take])
        del self._buffered[:take]
        return take

    def take_converted(self) -> list[Any]:
        self.sync_call_threads.append(threading.get_ident())
        rows, self._converted = self._converted, []
        return rows


def test_native_iterator_converts_rows_on_the_event_loop_thread():
    reader = _FakeBatchReader([[1, 2], [3, 4]])

    async def drain() -> tuple[list[int], int]:
        rows = [row async for row in NativeAsyncArrowRowIterator(reader)]
        return rows, threading.get_ident()

    rows, loop_thread = asyncio.run(drain())

    assert rows == [1, 2, 3, 4]
    assert set(reader.sync_call_threads) == {loop_thread}


def test_native_iterator_fetch_all_spans_batches():
    reader = _FakeBatchReader([[1, 2], [3, 4]])

    rows = asyncio.run(NativeAsyncArrowRowIterator(reader).fetch_all())

    assert rows == [1, 2, 3, 4]
    assert set(reader.sync_call_threads) == {threading.get_ident()}


def test_native_iterator_fetch_many_stops_at_size_and_resumes():
    reader = _FakeBatchReader([[1, 2], [3, 4]])
    iterator = NativeAsyncArrowRowIterator(reader)

    async def drain() -> list[list[int]]:
        return [await iterator.fetch_many(3), await iterator.fetch_many(3)]

    assert asyncio.run(drain()) == [[1, 2, 3], [4]]


def test_native_iterator_fetch_next_returns_default_when_exhausted():
    iterator = NativeAsyncArrowRowIterator(_FakeBatchReader([]))

    assert asyncio.run(iterator.fetch_next("done")) == "done"


def test_create_async_row_iterator_from_stream_ptr_wraps_sync_iterator():
    mock_sync = MagicMock(name="sync_iterator")
    mock_wrapper = MagicMock(name="async_wrapper")
    context = ArrowConverterContext(timezone="UTC")

    with (
        patch(
            "snowflake.connector._internal.arrow_stream_async.create_row_iterator",
            return_value=mock_sync,
        ) as mock_create,
        patch(
            "snowflake.connector._internal.arrow_stream_async.AsyncArrowStreamIterator",
            return_value=mock_wrapper,
        ) as mock_wrap,
    ):
        result = create_async_row_iterator_from_stream_ptr(
            99,
            context=context,
            use_dict_result=True,
        )

    assert result is mock_wrapper
    mock_create.assert_called_once_with(
        99,
        context=context,
        use_dict_result=True,
        use_numpy=False,
    )
    mock_wrap.assert_called_once_with(mock_sync)
