"""Distributed fetch tests (Python-specific).

Tests cursor.get_result_batches() with true distributed-style processing:
pickle each batch individually, spin up threads that each unpickle their
batch and iterate rows in parallel without reconnecting. A separate case
covers reconnection via a fresh connection after unpickling. Another case
verifies pickled TIMESTAMP_LTZ conversion retains the session timezone.
Verifies batch metadata (rowcount, sizes) and data correctness.
"""

from __future__ import annotations

import pickle

from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timedelta, timezone

import pytest
import pytz

from tests.e2e.types.utils import assert_connection_is_open, assert_timezone


LARGE_RESULT_SET_ROW_COUNT = 100_000
SESSION_TZ_NAME = "America/New_York"
SESSION_TZ = pytz.timezone(SESSION_TZ_NAME)
SEQUENTIAL_BASE_UTC = datetime(2024, 1, 1, 0, 0, 0, tzinfo=timezone.utc)


def sequential_timestamp(i: int) -> datetime:
    """Expected TIMESTAMP_LTZ value for DATEADD(second, i, base) under SESSION_TZ."""
    return (SEQUENTIAL_BASE_UTC + timedelta(seconds=i)).astimezone(SESSION_TZ)


class TestDistributedFetch:
    """Tests for cursor.get_result_batches()."""

    def test_should_fetch_all_rows_when_batches_are_pickled_and_fetched_in_parallel_threads(
        self, execute_query, cursor
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
            # And Each thread deserializes its batch and iterates rows without reconnecting
            def _fetch_batch_rows(pickled_batch: bytes) -> list[int]:
                restored_batch = pickle.loads(pickled_batch)
                return [row[0] for row in restored_batch.create_iter()]

            futures = [pool.submit(_fetch_batch_rows, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                all_ids.extend(future.result())

        # Then The combined row count across all threads should be 100000
        assert len(all_ids) == LARGE_RESULT_SET_ROW_COUNT

        # And All fetched ids from 0 to 99999 should be present exactly once
        assert sorted(all_ids) == list(range(LARGE_RESULT_SET_ROW_COUNT))

    def test_should_fetch_all_rows_when_batches_are_pickled_and_reconnected_in_parallel_threads(
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

        # And A thread pool fetches each batch after opening a fresh connection
        all_ids: list[int] = []
        with ThreadPoolExecutor(max_workers=4) as pool:

            def _fetch_batch_rows_with_reconnect(pickled_batch: bytes) -> list[int]:
                restored_batch = pickle.loads(pickled_batch)
                with connection_factory() as fresh_conn:
                    return [row[0] for row in restored_batch.create_iter(connection=fresh_conn)]

            futures = [pool.submit(_fetch_batch_rows_with_reconnect, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                all_ids.extend(future.result())

        # Then The combined row count across all threads should be 100000
        assert len(all_ids) == LARGE_RESULT_SET_ROW_COUNT
        assert sorted(all_ids) == list(range(LARGE_RESULT_SET_ROW_COUNT))


class TestDistributedFetchTimestampLtz:
    """Pickled ResultBatch fetch must retain session timezone for TIMESTAMP_LTZ conversion."""

    @pytest.fixture(autouse=True)
    def _set_session_timezone(self, cursor):
        cursor.execute(f"ALTER SESSION SET TIMEZONE = '{SESSION_TZ_NAME}'")
        yield
        cursor.execute("ALTER SESSION UNSET TIMEZONE")

    def test_should_preserve_session_timezone_when_pickled_batches_fetch_timestamp_ltz_without_connection(
        self, execute_query, cursor
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query generating 100000 TIMESTAMP_LTZ values is executed
        cursor.execute(
            "SELECT DATEADD(second, seq4(), '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) AS ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v"
        )

        # And get_result_batches is called
        batches = cursor.get_result_batches()
        assert batches is not None
        assert len(batches) >= 2, "Expected at least an inline batch and one remote batch"

        # And Each batch is individually serialized with pickle
        pickled_batches = [pickle.dumps(batch) for batch in batches]

        # And A thread pool deserializes each batch and iterates rows without reconnecting
        all_timestamps: list[datetime] = []
        with ThreadPoolExecutor(max_workers=4) as pool:

            def _fetch_batch_timestamps(pickled_batch: bytes) -> list[datetime]:
                restored_batch = pickle.loads(pickled_batch)
                return [row[0] for row in restored_batch.create_iter()]

            futures = [pool.submit(_fetch_batch_timestamps, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                all_timestamps.extend(future.result())

        # Then The combined row count across all threads should be 100000
        assert len(all_timestamps) == LARGE_RESULT_SET_ROW_COUNT

        # And Every TIMESTAMP_LTZ value should use the session timezone America/New_York
        assert_timezone(all_timestamps, expected_tz=SESSION_TZ_NAME)

        # And All values from second 0 to 99999 should be present exactly once
        assert sorted(all_timestamps) == [sequential_timestamp(i) for i in range(LARGE_RESULT_SET_ROW_COUNT)]
