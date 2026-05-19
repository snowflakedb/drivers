"""Unit tests for AsyncResultBatch."""

from __future__ import annotations

import pickle

from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    RemoteChunk,
    ResultChunk,
)
from snowflake.connector.aio._result_batch import AsyncResultBatch
from snowflake.connector.errors import InterfaceError
from snowflake.connector.result_batch import IterTableStructure, IterUnit
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_description(*names: str) -> list:
    return [SimpleNamespace(name=n) for n in names]


def _make_batch(connection=None, description=None) -> AsyncResultBatch:
    return AsyncResultBatch(
        chunk=ResultChunk(),
        description=description or _make_description("ID"),
        connection=connection,
    )


# ---------------------------------------------------------------------------
# from_chunks
# ---------------------------------------------------------------------------


class TestFromChunks:
    def test_returns_none_when_chunks_is_none(self):
        assert AsyncResultBatch.from_chunks(None, _make_description("ID"), MagicMock()) is None

    def test_returns_none_when_description_is_none(self):
        assert AsyncResultBatch.from_chunks([ResultChunk()], None, MagicMock()) is None

    def test_returns_list_of_async_batches(self):
        chunks = [ResultChunk(), ResultChunk(), ResultChunk()]
        desc = _make_description("A", "B")
        conn = MagicMock()
        batches = AsyncResultBatch.from_chunks(chunks, desc, conn)
        assert len(batches) == 3
        for batch in batches:
            assert isinstance(batch, AsyncResultBatch)
            assert batch.connection is conn

    def test_returns_empty_list_for_empty_chunks(self):
        assert AsyncResultBatch.from_chunks([], _make_description("ID"), MagicMock()) == []


# ---------------------------------------------------------------------------
# Properties
# ---------------------------------------------------------------------------


class TestProperties:
    def test_column_names(self):
        desc = _make_description("COL_A", "COL_B")
        batch = _make_batch(description=desc)
        assert batch.column_names == ["COL_A", "COL_B"]

    def test_connection_getter_and_setter(self):
        batch = _make_batch()
        assert batch.connection is None
        conn = MagicMock()
        batch.connection = conn
        assert batch.connection is conn

    def test_rowcount_defaults_to_zero(self):
        assert _make_batch().rowcount == 0

    def test_rowcount_from_chunk(self):
        chunk = ResultChunk(row_count=42)
        batch = AsyncResultBatch(chunk=chunk, description=_make_description("ID"), connection=None)
        assert batch.rowcount == 42

    def test_compressed_size_none_when_unset(self):
        assert _make_batch().compressed_size is None

    def test_compressed_size_from_chunk(self):
        chunk = ResultChunk(remote=RemoteChunk(url="http://example.com", compressed_size=1024))
        batch = AsyncResultBatch(chunk=chunk, description=_make_description("ID"), connection=None)
        assert batch.compressed_size == 1024

    def test_uncompressed_size_none_when_unset(self):
        assert _make_batch().uncompressed_size is None

    def test_uncompressed_size_from_chunk(self):
        chunk = ResultChunk(remote=RemoteChunk(url="http://example.com", uncompressed_size=4096))
        batch = AsyncResultBatch(chunk=chunk, description=_make_description("ID"), connection=None)
        assert batch.uncompressed_size == 4096


# ---------------------------------------------------------------------------
# _resolve_connection
# ---------------------------------------------------------------------------


class TestResolveConnection:
    def test_prefers_explicit_connection(self):
        stored = MagicMock()
        explicit = MagicMock()
        batch = _make_batch(connection=stored)
        assert batch._resolve_connection(explicit) is explicit

    def test_falls_back_to_stored_connection(self):
        stored = MagicMock()
        batch = _make_batch(connection=stored)
        assert batch._resolve_connection() is stored

    def test_raises_when_no_connection(self):
        batch = _make_batch()
        with pytest.raises(InterfaceError, match="not connected"):
            batch._resolve_connection()


# ---------------------------------------------------------------------------
# populate_data (async)
# ---------------------------------------------------------------------------


class TestPopulateData:
    @pytest.mark.asyncio
    async def test_populate_data_caches_stream_ptr(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        with patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=12345):
            result = await batch.populate_data()
        assert result is batch
        assert batch._arrow_stream_ptr == 12345

    @pytest.mark.asyncio
    async def test_populate_data_uses_explicit_connection(self):
        batch = _make_batch()
        explicit = MagicMock()
        with patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=1) as mock_fetch:
            await batch.populate_data(connection=explicit)
        mock_fetch.assert_awaited_once_with(explicit)


# ---------------------------------------------------------------------------
# _take_arrow_stream_ptr (async)
# ---------------------------------------------------------------------------


class TestTakeArrowStreamPtr:
    @pytest.mark.asyncio
    async def test_fetches_on_first_call(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        with patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=42):
            ptr = await batch._take_arrow_stream_ptr(conn)
        assert ptr == 42
        assert batch._arrow_stream_ptr is None  # consumed

    @pytest.mark.asyncio
    async def test_uses_cached_ptr(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        batch._arrow_stream_ptr = 99
        with patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock) as mock_fetch:
            ptr = await batch._take_arrow_stream_ptr(conn)
        mock_fetch.assert_not_awaited()
        assert ptr == 99
        assert batch._arrow_stream_ptr is None


# ---------------------------------------------------------------------------
# create_iter (async)
# ---------------------------------------------------------------------------


class TestCreateIter:
    @pytest.mark.asyncio
    async def test_row_unit_calls_create_row_iterator(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=0),
            patch("snowflake.connector.aio._result_batch.create_row_iterator") as mock_iter,
        ):
            mock_iter.return_value = iter([(1,), (2,)])
            result = await batch.create_iter(iter_unit=IterUnit.ROW_UNIT)
            rows = list(result)
        assert rows == [(1,), (2,)]
        mock_iter.assert_called_once_with(0, connection=conn, use_dict_result=False)

    @pytest.mark.asyncio
    async def test_row_unit_with_dict_result(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=0),
            patch("snowflake.connector.aio._result_batch.create_row_iterator") as mock_iter,
        ):
            mock_iter.return_value = iter([{"id": 1}])
            await batch.create_iter(iter_unit=IterUnit.ROW_UNIT, use_dict_result=True)
        mock_iter.assert_called_once_with(0, connection=conn, use_dict_result=True)

    @pytest.mark.asyncio
    async def test_table_unit_arrow(self):
        batch = _make_batch(connection=MagicMock())
        sentinel = MagicMock()
        with patch.object(batch, "to_arrow", new_callable=AsyncMock, return_value=sentinel):
            result = list(await batch.create_iter(iter_unit=IterUnit.TABLE_UNIT, structure=IterTableStructure.ARROW))
        assert result == [sentinel]

    @pytest.mark.asyncio
    async def test_table_unit_pandas(self):
        batch = _make_batch(connection=MagicMock())
        sentinel = MagicMock()
        with patch.object(batch, "to_pandas", new_callable=AsyncMock, return_value=sentinel):
            result = list(await batch.create_iter(iter_unit=IterUnit.TABLE_UNIT, structure=IterTableStructure.PANDAS))
        assert result == [sentinel]


# ---------------------------------------------------------------------------
# to_arrow / to_pandas (async)
# ---------------------------------------------------------------------------


class TestToArrow:
    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    @pytest.mark.asyncio
    async def test_to_arrow_uses_explicit_connection(self):
        batch = _make_batch()
        explicit = MagicMock()
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=0) as mock_fetch,
            patch("snowflake.connector.aio._result_batch.create_table_iterator"),
            patch("snowflake.connector.aio._result_batch.collect_arrow_table"),
        ):
            await batch.to_arrow(connection=explicit)
        mock_fetch.assert_awaited_once_with(explicit)

    @pytest.mark.asyncio
    async def test_to_arrow_raises_without_connection(self):
        batch = _make_batch()
        with pytest.raises(InterfaceError):
            await batch.to_arrow()

    @pytest.mark.asyncio
    async def test_to_arrow_forwards_params(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=0),
            patch("snowflake.connector.aio._result_batch.create_table_iterator") as mock_table_iter,
            patch("snowflake.connector.aio._result_batch.collect_arrow_table"),
        ):
            await batch.to_arrow(number_to_decimal=True, force_microsecond_precision=True)
        mock_table_iter.assert_called_once_with(
            0, connection=conn, number_to_decimal=True, force_microsecond_precision=True
        )


class TestToPandas:
    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    @pytest.mark.asyncio
    async def test_to_pandas_delegates_to_to_arrow(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        mock_table = MagicMock()
        with patch.object(batch, "to_arrow", new_callable=AsyncMock, return_value=mock_table):
            await batch.to_pandas(connection=conn, number_to_decimal=True, force_microsecond_precision=True)
        mock_table.to_pandas.assert_called_once()


# ---------------------------------------------------------------------------
# async for iteration
# ---------------------------------------------------------------------------


class TestAsyncIteration:
    @pytest.mark.asyncio
    async def test_async_for_yields_rows(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", new_callable=AsyncMock, return_value=0),
            patch("snowflake.connector.aio._result_batch.create_row_iterator") as mock_iter,
        ):
            mock_iter.return_value = iter([(1,), (2,), (3,)])
            rows = [row async for row in batch]
        assert rows == [(1,), (2,), (3,)]


# ---------------------------------------------------------------------------
# Pickle
# ---------------------------------------------------------------------------


class TestPickle:
    def test_pickle_round_trip_preserves_description(self):
        desc = _make_description("X", "Y")
        batch = _make_batch(connection=MagicMock(), description=desc)
        restored = pickle.loads(pickle.dumps(batch))
        assert restored.column_names == ["X", "Y"]

    def test_pickle_clears_connection(self):
        batch = _make_batch(connection=MagicMock())
        restored = pickle.loads(pickle.dumps(batch))
        assert restored.connection is None
