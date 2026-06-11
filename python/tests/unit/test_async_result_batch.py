"""Unit tests for AsyncResultBatch."""

from types import SimpleNamespace
from unittest.mock import MagicMock

from snowflake.connector._async.result_batch import AsyncResultBatch
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ResultChunk


def _make_description(*names: str) -> list:
    return [SimpleNamespace(name=n) for n in names]


class TestAsyncFromChunks:
    def test_returns_list_of_async_batches(self):
        chunks = [ResultChunk(), ResultChunk()]
        desc = _make_description("A")
        conn = MagicMock()
        batches = AsyncResultBatch.from_chunks(chunks, desc, conn)

        assert len(batches) == 2
        for batch in batches:
            assert isinstance(batch, AsyncResultBatch)
            assert batch.connection is conn

    def test_returns_none_when_chunks_unavailable(self):
        assert AsyncResultBatch.from_chunks(None, _make_description("ID"), MagicMock()) is None
