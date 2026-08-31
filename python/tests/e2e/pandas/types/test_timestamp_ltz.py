"""TIMESTAMP_LTZ type tests for Universal Driver -- pandas consumer.

Arrow timestamp -> pandas ``datetime64[ns, tz]`` dtype.  The driver converts
TIMESTAMP_LTZ values to the session timezone and returns tz-aware
``pd.Timestamp`` objects.

The session timezone is explicitly set to America/New_York (a non-UTC zone)
so that tests verify the driver actually propagates the session timezone
to the result rather than silently falling back to UTC.

SQL string literals use explicit '+00:00' UTC offsets so input values are
deterministic; expected values match ``python/tests/e2e/types/test_timestamp_ltz.py``
(instants converted to the session timezone).

NULL values are ``pd.NaT``.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pandas as pd
import pytest
import pytz

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_dtypes,
    assert_timezone,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_datetime64_tz,
)
from tests.e2e.types.utils import assert_sequential_values, batch_insert


# Constants aligned with python/tests/e2e/types/test_timestamp_ltz.py
SESSION_TZ_NAME = "America/New_York"
SESSION_TZ = pytz.timezone(SESSION_TZ_NAME)

TS_2024_JAN_STR = "2024-01-15 10:30:00 +00:00"
TS_2024_JUN_STR = "2024-06-20 14:45:30 +00:00"
TS_EPOCH_STR = "1970-01-01 00:00:00 +00:00"
TS_WITH_MICROSECONDS_STR = "2024-01-15 10:30:00.123456 +00:00"

TS_2024_JAN = datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone.utc).astimezone(SESSION_TZ)
TS_2024_JUN = datetime(2024, 6, 20, 14, 45, 30, tzinfo=timezone.utc).astimezone(SESSION_TZ)
TS_EPOCH = datetime(1970, 1, 1, 0, 0, 0, tzinfo=timezone.utc).astimezone(SESSION_TZ)
TS_WITH_MICROSECONDS = datetime(2024, 1, 15, 10, 30, 0, 123456, tzinfo=timezone.utc).astimezone(SESSION_TZ)

LARGE_RESULT_SET_SIZE = 50_000
SEQUENTIAL_BASE_UTC = datetime(2024, 1, 1, 0, 0, 0, tzinfo=timezone.utc)


def sequential_timestamp(i):
    """Map row index to expected TIMESTAMP_LTZ value in the session timezone."""
    return (SEQUENTIAL_BASE_UTC + timedelta(seconds=i)).astimezone(SESSION_TZ)


@pytest.fixture(autouse=True)
def _set_session_timezone(cursor):
    """Set session timezone to a non-UTC zone for all tests in this module."""
    cursor.execute(f"ALTER SESSION SET TIMEZONE = '{SESSION_TZ_NAME}'")


LITERAL_SELECT_TEST_CASES = [
    ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN]),
    ("epoch", [TS_EPOCH_STR], [TS_EPOCH]),
    ("microseconds", [TS_WITH_MICROSECONDS_STR], [TS_WITH_MICROSECONDS]),
]

TABLE_SELECT_TEST_CASES = [
    ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN]),
    ("epoch", [TS_EPOCH_STR, TS_2024_JAN_STR], [TS_EPOCH, TS_2024_JAN]),
    ("null", [None, TS_2024_JAN_STR], [TS_2024_JAN, pd.NaT]),
]


class TestFetchPandasTimestampLtzTypeCasting:
    """Type-casting coverage for TIMESTAMP_LTZ via fetch_pandas_all."""

    def test_should_cast_timestamp_ltz_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
        df = execute_and_fetch(cursor, f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_LTZ")

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_datetime64_tz])
        val = get_row(df, 0)[0]
        assert val == TS_2024_JAN
        # And Values should have timezone info
        assert_timezone((val,), SESSION_TZ_NAME)


class TestFetchPandasTimestampLtzLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    @pytest.mark.parametrize(
        "values_name,query_values,expected_values",
        LITERAL_SELECT_TEST_CASES,
        ids=[c[0] for c in LITERAL_SELECT_TEST_CASES],
    )
    def test_should_select_timestamp_ltz_values(self, cursor, values_name, query_values, expected_values):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"'{v}'::TIMESTAMP_LTZ" for v in query_values)
        df = execute_and_fetch(cursor, f"SELECT {select_cols}")

        # Then Result should contain timestamps <expected_values>
        row = get_row(df, 0)
        assert_dtypes(df, [is_datetime64_tz for _ in expected_values])
        assert_timezone(row, SESSION_TZ_NAME)
        assert row == expected_values

    def test_should_handle_null_values_for_timestamp_ltz(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ",
        )

        # Then Result should contain [2024-01-15 10:30:00 UTC, NULL]
        assert_dtypes(df, [is_datetime64_tz, is_datetime64_tz])
        row = get_row(df, 0)
        assert_timezone((row[0],), SESSION_TZ_NAME)
        assert row[0] == TS_2024_JAN
        assert row[1] is pd.NaT

    def test_should_download_large_result_set_with_multiple_chunks_for_timestamp_ltz(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,
        #   '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            "'2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY ts",
        )

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        col = get_column(combined, 0)
        assert_timezone(col, SESSION_TZ_NAME)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


class TestFetchPandasTimestampLtzTable:
    """Table-based scenarios via fetch_pandas_all."""

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values",
        TABLE_SELECT_TEST_CASES,
        ids=[c[0] for c in TABLE_SELECT_TEST_CASES],
    )
    def test_should_select_values_from_table_for_timestamp_ltz(
        self, execute_query, cursor, tmp_schema, values_name, insert_values, expected_values
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_LTZ column exists with values <insert_values>
        table_name = f"{tmp_schema}.pd_timestamp_ltz_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_LTZ)")
        batch_insert(execute_query, table_name, insert_values, quote_strings=True)

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain timestamps <expected_values>
        col = get_column(df, 0)
        assert_dtypes(df, [is_datetime64_tz])
        assert col == expected_values
        assert_timezone(col, SESSION_TZ_NAME, can_be_none=True)

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_timestamp_ltz(
        self, execute_query, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
        table_name = f"{tmp_schema}.pd_large_timestamp_ltz_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_LTZ)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        col = get_column(combined, 0)
        assert_timezone(col, SESSION_TZ_NAME)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


@with_paramstyle("qmark")
class TestFetchPandasTimestampLtzBinding:
    """Parameter-binding scenarios via fetch_pandas_all.

    Matches ``python/tests/e2e/types/test_timestamp_ltz.py``: the driver binds datetimes as
    TIMESTAMP_NTZ, then ``?::TIMESTAMP_LTZ`` interprets them as session-local wall clocks.
    Returned UTC instants need **not** match ``TS_2024_JAN`` / ``TS_2024_JUN`` (those come
    from literal SQL under different encoding rules). We only verify dtypes and session-TZ
    representation—never UTC equality to the bound constants.
    """

    def test_should_select_timestamp_ltz_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
        df = execute_and_fetch(
            cursor,
            "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ",
            params=(TS_2024_JAN, TS_2024_JUN),
        )

        # Then Result should contain the bound timestamps
        row = get_row(df, 0)
        assert_dtypes(df, [is_datetime64_tz, is_datetime64_tz])
        assert_timezone(row, SESSION_TZ_NAME)

    def test_should_select_null_timestamp_ltz_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
        df = execute_and_fetch(cursor, "SELECT ?::TIMESTAMP_LTZ", params=(None,))

        # Then Result should contain [NULL]
        assert_dtypes(df, [is_datetime64_tz])
        assert get_row(df, 0)[0] is pd.NaT

    def test_should_insert_timestamp_ltz_using_parameter_binding(
        self, execute_query, executemany_insert, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_LTZ column exists
        table_name = f"{tmp_schema}.pd_timestamp_ltz_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_LTZ)")

        # When Timestamp values are bulk-inserted using multirow binding
        test_values = [
            (TS_2024_JAN,),
            (TS_2024_JUN,),
            (None,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then SELECT should return the same values in any order
        assert_dtypes(df, [is_datetime64_tz])
        col = get_column(df, 0)
        assert_timezone((col[0], col[1]), SESSION_TZ_NAME)
        assert col[2] is pd.NaT


class TestFetchPandasTimestampLtzNegativeEpoch:
    """Sub-second TIMESTAMP_LTZ instants just before the Unix epoch, via fetch_pandas_all.

    The server pre-floors the seconds-since-epoch decimal, so Arrow surfaces
    1969-12-31 23:59:59.999999999 with its borrowed second already applied and
    pandas retains the full nanosecond fraction. TIMESTAMP_LTZ is an absolute
    instant, so assertions compare the UTC instant and are independent of the
    session timezone. The two scales cover both LTZ builders: scale 9 (struct
    epoch+fraction) and scale 3 (single combined Int64).
    """

    @pytest.mark.parametrize(
        "query,expected",
        [
            ("SELECT '1969-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_LTZ(9)", "1969-12-31 23:59:59.999999999+00:00"),
            ("SELECT '1969-12-31 23:59:58.5 +00:00'::TIMESTAMP_LTZ(3)", "1969-12-31 23:59:58.5+00:00"),
        ],
    )
    def test_should_select_sub_second_timestamp_ltz_values_before_epoch(self, cursor, query, expected):
        # Given Snowflake client is logged in
        pass

        # When Sub-second timestamp_ltz values before the epoch are selected
        df = execute_and_fetch(cursor, query)

        # Then Result should contain the expected sub-second values before the epoch
        assert_dtypes(df, [is_datetime64_tz])
        val = pd.Timestamp(get_row(df, 0)[0])
        assert val.tzinfo is not None
        assert val.tz_convert("UTC") == pd.Timestamp(expected).tz_convert("UTC")
