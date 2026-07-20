"""Distributed fetch tests.

Tests cursor.get_result_batches() with true distributed-style processing:
split the result set into independently serializable partitions, serialize
(pickle) each one on its own, then spin up threads that each deserialize their
partition and iterate rows in parallel without sharing the original session. A
separate case covers reconnection via a fresh connection after unpickling.
Another case verifies pickled TIMESTAMP_LTZ conversion retains the session
timezone. Verifies partition metadata (rowcount, sizes) and data correctness.
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

    def test_should_fetch_all_rows_when_partitions_fetched_in_parallel_threads(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And the result set is split into independently serializable partitions
        batches = cursor.get_result_batches()
        assert batches is not None

        # Then there should be at least two partitions
        assert len(batches) >= 2, "Expected at least an inline batch and one remote batch"

        # When each partition is serialized and fetched on its own worker thread without a live session
        pickled_batches = [pickle.dumps(batch) for batch in batches]
        all_ids: list[int] = []
        with ThreadPoolExecutor(max_workers=4) as pool:

            def _fetch_batch_rows(pickled_batch: bytes) -> list[int]:
                restored_batch = pickle.loads(pickled_batch)
                return [row[0] for row in restored_batch.create_iter()]

            futures = [pool.submit(_fetch_batch_rows, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                all_ids.extend(future.result())

        # Then the combined row count across all threads should be 100000
        assert len(all_ids) == LARGE_RESULT_SET_ROW_COUNT

        # And all ids from 0 to 99999 should be present exactly once
        assert sorted(all_ids) == list(range(LARGE_RESULT_SET_ROW_COUNT))

    def test_should_preserve_row_count_and_data_sizes_across_partition_split(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And the result set is split into independently serializable partitions
        batches = cursor.get_result_batches()
        assert batches is not None
        assert len(batches) >= 2, "Expected at least an inline batch and one remote batch"

        # Then the sum of the partition row counts should be 100000
        assert sum(b.rowcount for b in batches) == LARGE_RESULT_SET_ROW_COUNT

        # The inline partition carries no compressed payload (None); every remote partition reports
        # positive compressed and uncompressed sizes, so together they account for the whole result.
        # And the aggregate compressed and uncompressed data sizes should be preserved across the split
        inline_batch = batches[0]
        assert inline_batch.compressed_size is None
        assert inline_batch.uncompressed_size is None
        for batch in batches[1:]:
            assert batch.compressed_size is not None and batch.compressed_size > 0
            assert batch.uncompressed_size is not None and batch.uncompressed_size > 0

    def test_should_fetch_all_rows_when_batches_are_pickled_and_reconnected_in_parallel_threads(
        self, execute_query, cursor, connection_factory
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And the result set is split into independently serializable partitions
        batches = cursor.get_result_batches()
        assert batches is not None

        # When each partition is serialized and fetched on its own worker thread after opening a fresh session
        pickled_batches = [pickle.dumps(batch) for batch in batches]
        all_ids: list[int] = []
        with ThreadPoolExecutor(max_workers=4) as pool:

            def _fetch_batch_rows_with_reconnect(pickled_batch: bytes) -> list[int]:
                restored_batch = pickle.loads(pickled_batch)
                with connection_factory() as fresh_conn:
                    return [row[0] for row in restored_batch.create_iter(connection=fresh_conn)]

            futures = [pool.submit(_fetch_batch_rows_with_reconnect, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                all_ids.extend(future.result())

        # Then the combined row count across all threads should be 100000
        assert len(all_ids) == LARGE_RESULT_SET_ROW_COUNT

        # And all ids from 0 to 99999 should be present exactly once
        assert sorted(all_ids) == list(range(LARGE_RESULT_SET_ROW_COUNT))


class TestDistributedFetchTimestampLtz:
    """Pickled partition fetch must retain session timezone for TIMESTAMP_LTZ conversion."""

    @pytest.fixture(autouse=True)
    def _set_session_timezone(self, cursor):
        cursor.execute(f"ALTER SESSION SET TIMEZONE = '{SESSION_TZ_NAME}'")
        yield
        cursor.execute("ALTER SESSION UNSET TIMEZONE")

    def test_should_preserve_session_timezone_for_timestamp_ltz_fetched_from_serializable_without_a_live_session(
        self, execute_query, cursor
    ):
        # Given Snowflake client is logged in with a non-default session timezone
        assert_connection_is_open(execute_query)

        # When a query returning TIMESTAMP_LTZ values is executed
        cursor.execute(
            "SELECT DATEADD(second, seq4(), '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) AS ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v"
        )

        # And the result set is split into independently serializable partitions
        batches = cursor.get_result_batches()
        assert batches is not None
        assert len(batches) >= 2, "Expected at least an inline batch and one remote batch"

        # And each partition is serialized and fetched without a live session
        pickled_batches = [pickle.dumps(batch) for batch in batches]
        all_timestamps: list[datetime] = []
        with ThreadPoolExecutor(max_workers=4) as pool:

            def _fetch_batch_timestamps(pickled_batch: bytes) -> list[datetime]:
                restored_batch = pickle.loads(pickled_batch)
                return [row[0] for row in restored_batch.create_iter()]

            futures = [pool.submit(_fetch_batch_timestamps, pb) for pb in pickled_batches]
            for future in as_completed(futures):
                all_timestamps.extend(future.result())

        # Then the fetched timestamp values should match those rendered by the originating session
        assert len(all_timestamps) == LARGE_RESULT_SET_ROW_COUNT
        assert_timezone(all_timestamps, expected_tz=SESSION_TZ_NAME)
        assert sorted(all_timestamps) == [sequential_timestamp(i) for i in range(LARGE_RESULT_SET_ROW_COUNT)]
