"""Distributed fetch tests (Python-specific).

Tests cursor.get_result_batches() with true distributed-style processing:
pickle each batch individually, spin up threads that each unpickle their
batch, open a fresh connection, and iterate rows in parallel.
"""

from __future__ import annotations

import pickle

from concurrent.futures import ThreadPoolExecutor, as_completed

from tests.e2e.types.utils import assert_connection_is_open


LARGE_RESULT_SET_ROW_COUNT = 100_000


class TestDistributedFetch:
    """Tests for cursor.get_result_batches()."""

    def test_should_fetch_all_rows_when_batches_are_pickled_and_fetched_in_parallel_threads(
        self, execute_query, cursor, connection_factory
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And get_result_batches is called
        batches = cursor.get_result_batches()
        assert batches is not None

        # And Each batch is individually serialized with pickle
        pickled_batches = [pickle.dumps(batch) for batch in batches]

        # And A thread pool is started with up to 4 workers
        row_counts: list[int] = []
        with ThreadPoolExecutor(max_workers=min(4, len(pickled_batches))) as pool:
            # And Each thread deserializes its batch, opens a fresh connection, and iterates rows
            def _fetch_batch_rows(pickled_batch: bytes) -> int:
                restored_batch = pickle.loads(pickled_batch)
                with connection_factory() as conn:
                    return sum(1 for _row in restored_batch.create_iter(connection=conn))

            futures = [pool.submit(_fetch_batch_rows, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                row_counts.append(future.result())

        # Then The combined row count across all threads should be 100000
        assert sum(row_counts) == LARGE_RESULT_SET_ROW_COUNT
