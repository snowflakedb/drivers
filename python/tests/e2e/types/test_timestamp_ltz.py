"""TIMESTAMP_LTZ type tests for Universal Driver.

TIMESTAMP_LTZ (Local Time Zone) stores timestamp with local timezone.
Values are stored in UTC and converted to the session timezone on retrieval.
Python type: datetime with tzinfo set to the session timezone.

The session timezone is explicitly set to America/New_York (a non-UTC zone)
so that tests verify the driver actually propagates the session timezone
to the result rather than silently falling back to UTC.

SQL string literals use explicit '+00:00' UTC offsets so input values are
deterministic; expected values are expressed in America/New_York.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest
import pytz

from ...conftest import with_paramstyle
from .utils import assert_datetime_type, assert_sequential_values, assert_timezone, batch_insert


SESSION_TZ_NAME = "America/New_York"
SESSION_TZ = pytz.timezone(SESSION_TZ_NAME)

# =============================================================================
# SQL STRING REPRESENTATIONS (with explicit UTC offset)
# =============================================================================
TS_2024_JAN_STR = "2024-01-15 10:30:00 +00:00"
TS_2024_JUN_STR = "2024-06-20 14:45:30 +00:00"
TS_EPOCH_STR = "1970-01-01 00:00:00 +00:00"
TS_WITH_MICROSECONDS_STR = "2024-01-15 10:30:00.123456 +00:00"

# =============================================================================
# EXPECTED DATETIME VALUES in America/New_York
# Constructed from the UTC instants then converted to the session timezone.
# Jan 15 is EST (UTC-5): 10:30 UTC -> 05:30 EST
# Jun 20 is EDT (UTC-4): 14:45:30 UTC -> 10:45:30 EDT
# Epoch is EST (UTC-5): 00:00 UTC -> 1969-12-31 19:00 EST
# =============================================================================
TS_2024_JAN = datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone.utc).astimezone(SESSION_TZ)
TS_2024_JUN = datetime(2024, 6, 20, 14, 45, 30, tzinfo=timezone.utc).astimezone(SESSION_TZ)
TS_EPOCH = datetime(1970, 1, 1, 0, 0, 0, tzinfo=timezone.utc).astimezone(SESSION_TZ)
TS_WITH_MICROSECONDS = datetime(2024, 1, 15, 10, 30, 0, 123456, tzinfo=timezone.utc).astimezone(SESSION_TZ)

# =============================================================================
# LARGE RESULT SET
# =============================================================================
LARGE_RESULT_SET_SIZE = 50_000
SEQUENTIAL_BASE_UTC = datetime(2024, 1, 1, 0, 0, 0, tzinfo=timezone.utc)


def sequential_timestamp(i):
    """Transform index to expected sequential timestamp in session timezone."""
    return (SEQUENTIAL_BASE_UTC + timedelta(seconds=i)).astimezone(SESSION_TZ)


@pytest.fixture(autouse=True)
def _set_session_timezone(cursor):
    """Set session timezone to a non-UTC zone for all tests in this module."""
    cursor.execute(f"ALTER SESSION SET TIMEZONE = '{SESSION_TZ_NAME}'")


class TestTimestampLtzTypeCasting:
    """Tests for TIMESTAMP_LTZ type casting to appropriate type."""

    def test_should_cast_timestamp_ltz_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
        result = execute_query(f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_LTZ", single_row=True)

        # Then All values should be returned as appropriate type
        assert_datetime_type(result)

        # And Values should have timezone info
        assert_timezone(result, expected_tz=SESSION_TZ_NAME)


class TestTimestampLtzLiteral:
    """Tests for TIMESTAMP_LTZ type using SELECT with literals (no tables)."""

    # Examples:
    #   | values       | query_values                                            | expected_values          |
    #   | basic        | 2024-01-15 10:30:00 +00:00, 2024-06-20 14:45:30 +00:00  | TS_2024_JAN, TS_2024_JUN |
    #   | epoch        | 1970-01-01 00:00:00 +00:00                               | TS_EPOCH                 |
    #   | microseconds | 2024-01-15 10:30:00.123456 +00:00                        | TS_WITH_MICROSECONDS     |
    LITERAL_SELECT_TEST_CASES = [
        ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], (TS_2024_JAN, TS_2024_JUN)),
        ("epoch", [TS_EPOCH_STR], (TS_EPOCH,)),
        ("microseconds", [TS_WITH_MICROSECONDS_STR], (TS_WITH_MICROSECONDS,)),
    ]

    @pytest.mark.parametrize(
        "values,query_values,expected_values",
        LITERAL_SELECT_TEST_CASES,
        ids=[c[0] for c in LITERAL_SELECT_TEST_CASES],
    )
    def test_should_select_timestamp_ltz_values(self, execute_query, values, query_values, expected_values):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"'{v}'::TIMESTAMP_LTZ" for v in query_values)
        result = execute_query(f"SELECT {select_cols}", single_row=True)

        # Then Result should contain timestamps <expected_values>
        assert_datetime_type(result)
        assert_timezone(result, expected_tz=SESSION_TZ_NAME)
        assert tuple(result) == expected_values

    def test_should_handle_null_values_for_timestamp_ltz(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
        result = execute_query(
            f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ",
            single_row=True,
        )

        # Then Result should contain [2024-01-15 10:30:00 UTC, NULL]
        assert_datetime_type(result, can_be_none=True)
        assert_timezone(result, expected_tz=SESSION_TZ_NAME, can_be_none=True)
        assert result == (TS_2024_JAN, None)

    def test_should_download_large_result_set_with_multiple_chunks_for_timestamp_ltz(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,
        #   '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        sql = (
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY 1"
        )
        rows = execute_query(sql)

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        values = [row[0] for row in rows]
        assert_datetime_type(values)
        assert_timezone(values, expected_tz=SESSION_TZ_NAME)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


class TestTimestampLtzTable:
    """Tests for TIMESTAMP_LTZ type using table operations."""

    TABLE_SELECT_TEST_CASES = [
        ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN], False),
        ("epoch", [TS_EPOCH_STR, TS_2024_JAN_STR], [TS_EPOCH, TS_2024_JAN], False),
        ("null", [None, TS_2024_JAN_STR], [TS_2024_JAN, None], True),
    ]

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values,can_be_none",
        TABLE_SELECT_TEST_CASES,
        ids=[c[0] for c in TABLE_SELECT_TEST_CASES],
    )
    def test_should_select_values_from_table_for_timestamp_ltz(
        self, execute_query, tmp_schema, values_name, insert_values, expected_values, can_be_none
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_LTZ column exists with values <insert_values>
        table_name = f"{tmp_schema}.timestamp_ltz_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_LTZ)")
        batch_insert(execute_query, table_name, insert_values, quote_strings=True)

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
        result = [row[0] for row in rows]

        # Then Result should contain timestamps <expected_values>
        assert_datetime_type(result, can_be_none=can_be_none)
        assert_timezone(result, expected_tz=SESSION_TZ_NAME, can_be_none=can_be_none)
        assert result == expected_values

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_timestamp_ltz(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
        table_name = f"{tmp_schema}.large_timestamp_ltz_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_LTZ)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC
        values = [row[0] for row in rows]
        assert_datetime_type(values)
        assert_timezone(values, expected_tz=SESSION_TZ_NAME)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


@with_paramstyle("qmark")
class TestTimestampLtzBinding:
    """Tests for TIMESTAMP_LTZ type using parameter binding.

    The driver binds datetimes as TIMESTAMP_NTZ (see PYTHON_TO_SNOWFLAKE_TYPE),
    so ?::TIMESTAMP_LTZ casts NTZ->LTZ treating the wall-clock time as session-local.
    Exact UTC values depend on session timezone, so we only verify types and counts.
    """

    def test_should_select_timestamp_ltz_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
        result = execute_query(
            "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ",
            (TS_2024_JAN, TS_2024_JUN),
            single_row=True,
        )

        # Then Result should contain the bound timestamps
        assert_datetime_type(result)
        assert_timezone(result, expected_tz=SESSION_TZ_NAME)
        assert len(result) == 2

    def test_should_select_null_timestamp_ltz_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
        result = execute_query("SELECT ?::TIMESTAMP_LTZ", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    def test_should_insert_timestamp_ltz_using_parameter_binding(self, execute_query, executemany_insert, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_LTZ column exists
        table_name = f"{tmp_schema}.timestamp_ltz_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_LTZ)")

        # When Timestamp values are bulk-inserted using multirow binding
        test_values = [
            (TS_2024_JAN,),
            (TS_2024_JUN,),
            (None,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
        result = [row[0] for row in rows]

        # Then SELECT should return the same values in any order
        non_null_results = [r for r in result if r is not None]
        null_results = [r for r in result if r is None]
        assert len(non_null_results) == 2
        assert len(null_results) == 1
        assert_datetime_type(non_null_results)
        assert_timezone(non_null_results, expected_tz=SESSION_TZ_NAME)
