"""Unit tests for the aio ResultBatch."""

from types import SimpleNamespace
from unittest.mock import MagicMock

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ResultChunk
from snowflake.connector.aio.result_batch import ResultBatch


def _make_description(*names: str) -> list:
    return [SimpleNamespace(name=n) for n in names]


class TestAsyncFromChunks:
    def test_returns_list_of_async_batches(self):
        chunks = [ResultChunk(), ResultChunk()]
        desc = _make_description("A")
        conn = MagicMock()
        batches = ResultBatch.from_chunks(chunks, desc, conn)

        assert len(batches) == 2
        for batch in batches:
            assert isinstance(batch, ResultBatch)
            assert batch.connection is conn

    def test_returns_none_when_chunks_unavailable(self):
        assert ResultBatch.from_chunks(None, _make_description("ID"), MagicMock()) is None
