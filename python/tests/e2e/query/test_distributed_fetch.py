"""Distributed fetch tests (Python-specific).

Tests cursor.get_result_batches() with true distributed-style processing:
pickle each batch individually, spin up threads that each unpickle their
batch, open a fresh connection, and iterate rows in parallel.
Verifies batch metadata (rowcount, sizes) and data correctness.
"""

from __future__ import annotations

import pickle

from concurrent.futures import ThreadPoolExecutor, as_completed

import pytest

from tests.e2e.types.utils import assert_connection_is_open


LARGE_RESULT_SET_ROW_COUNT = 100_000


class TestDistributedFetch:
    """Tests for cursor.get_result_batches()."""

    @pytest.mark.skip_for_json_result_set
    def test_should_fetch_all_rows_when_batches_are_pickled_and_fetched_in_parallel_threads(
        self, execute_query, cursor, connection
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And get_result_batches is called
        batches = cursor.get_result_batches()
        assert batches is not None
        assert len(batches) >= 2, "Expected at least an inline batch and one remote batch"

        # Then The sum of batch rowcounts should equal 100000
        assert sum(b.rowcount for b in batches) == LARGE_RESULT_SET_ROW_COUNT

        # And The inline batch should have None for compressed_size and uncompressed_size
        inline_batch = batches[0]
        assert inline_batch.compressed_size is None
        assert inline_batch.uncompressed_size is None

        # And Every remote batch should have positive compressed_size and uncompressed_size
        for batch in batches[1:]:
            assert batch.compressed_size is not None and batch.compressed_size > 0
            assert batch.uncompressed_size is not None and batch.uncompressed_size > 0

        # And Each batch is individually serialized with pickle
        pickled_batches = [pickle.dumps(batch) for batch in batches]

        # And A thread pool is started with up to 4 workers
        all_ids: list[int] = []
        with ThreadPoolExecutor(max_workers=4) as pool:
            # And Each thread deserializes its batch, opens a fresh connection, and iterates rows
            def _fetch_batch_rows(pickled_batch: bytes) -> list[int]:
                restored_batch = pickle.loads(pickled_batch)
                return [row[0] for row in restored_batch.create_iter(connection=connection)]

            futures = [pool.submit(_fetch_batch_rows, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                all_ids.extend(future.result())

        # Then The combined row count across all threads should be 100000
        assert len(all_ids) == LARGE_RESULT_SET_ROW_COUNT

        # And All fetched ids from 0 to 99999 should be present exactly once
        assert sorted(all_ids) == list(range(LARGE_RESULT_SET_ROW_COUNT))
