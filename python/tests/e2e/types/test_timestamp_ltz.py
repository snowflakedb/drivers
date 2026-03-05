"""TIMESTAMP_LTZ type tests for Universal Driver.

TIMESTAMP_LTZ (Local Time Zone) stores timestamp with local timezone.
Values are stored in UTC and converted to the session timezone on retrieval.
Python type: datetime with tzinfo set (not None).

All SQL string literals use explicit '+00:00' UTC offset so that expected
values are deterministic regardless of the Snowflake session timezone.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

from ...conftest import with_paramstyle
from .utils import assert_connection_is_open, assert_datetime_type, assert_sequential_values, batch_insert


# =============================================================================
# EXPECTED DATETIME VALUES (UTC)
# =============================================================================
TS_2024_JAN = datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone.utc)
TS_2024_JUN = datetime(2024, 6, 20, 14, 45, 30, tzinfo=timezone.utc)
TS_EPOCH = datetime(1970, 1, 1, 0, 0, 0, tzinfo=timezone.utc)
TS_WITH_MICROSECONDS = datetime(2024, 1, 15, 10, 30, 0, 123456, tzinfo=timezone.utc)

# =============================================================================
# SQL STRING REPRESENTATIONS (with explicit UTC offset)
# =============================================================================
TS_2024_JAN_STR = "2024-01-15 10:30:00 +00:00"
TS_2024_JUN_STR = "2024-06-20 14:45:30 +00:00"
TS_EPOCH_STR = "1970-01-01 00:00:00 +00:00"
TS_WITH_MICROSECONDS_STR = "2024-01-15 10:30:00.123456 +00:00"

# =============================================================================
# LARGE RESULT SET
# =============================================================================
LARGE_RESULT_SET_SIZE = 50_000
SEQUENTIAL_BASE = datetime(2024, 1, 1, 0, 0, 0, tzinfo=timezone.utc)


def to_utc(values):
    """Convert datetime values to UTC, preserving None."""
    return [v.astimezone(timezone.utc) if v is not None else None for v in values]


def sequential_timestamp(i):
    """Transform index to expected sequential UTC timestamp."""
    return SEQUENTIAL_BASE + timedelta(seconds=i)


def compare_ts_utc(actual, expected):
    """Compare timestamps by converting actual to UTC."""
    return actual.astimezone(timezone.utc) == expected


class TestTimestampLtzTypeCasting:
    """Tests for TIMESTAMP_LTZ type casting to appropriate type."""

    def test_should_cast_timestamp_ltz_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
        result = execute_query(f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_LTZ", single_row=True)

        # Then All values should be returned as appropriate type
        # And Values should have timezone info
        assert_datetime_type(result, require_tzinfo=True)


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

    def test_should_select_timestamp_ltz_values(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        for _name, query_values, expected_values in self.LITERAL_SELECT_TEST_CASES:
            # When Query "SELECT <query_values>" is executed
            select_cols = ", ".join(f"'{v}'::TIMESTAMP_LTZ" for v in query_values)
            result = execute_query(f"SELECT {select_cols}", single_row=True)

            # Then Result should contain timestamps <expected_values>
            assert_datetime_type(result, require_tzinfo=True)
            assert tuple(to_utc(result)) == expected_values

    def test_should_handle_null_values_for_timestamp_ltz(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
        result = execute_query(
            f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ",
            single_row=True,
        )

        # Then Result should contain [2024-01-15 10:30:00 UTC, NULL]
        assert_datetime_type(result, can_be_none=True, require_tzinfo=True)
        assert tuple(to_utc(result)) == (TS_2024_JAN, None)

    def test_should_download_large_result_set_with_multiple_chunks_for_timestamp_ltz(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

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
        assert_datetime_type(values, require_tzinfo=True)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp, compare=compare_ts_utc)


class TestTimestampLtzTable:
    """Tests for TIMESTAMP_LTZ type using table operations."""

    TABLE_SELECT_TEST_CASES = [
        ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN], False),
        ("epoch", [TS_EPOCH_STR, TS_2024_JAN_STR], [TS_EPOCH, TS_2024_JAN], False),
        ("null", [None, TS_2024_JAN_STR], [TS_2024_JAN, None], True),
    ]

    def test_should_select_values_from_table_for_timestamp_ltz(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        for values_name, insert_values, expected_values, can_be_none in self.TABLE_SELECT_TEST_CASES:
            # And Table with TIMESTAMP_LTZ column exists with values <insert_values>
            table_name = f"{tmp_schema}.timestamp_ltz_table_{values_name}"
            execute_query(f"CREATE TABLE {table_name} (col TIMESTAMP_LTZ)")
            batch_insert(execute_query, table_name, insert_values, quote_strings=True)

            # When Query "SELECT * FROM <table> ORDER BY col" is executed
            rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
            result = [row[0] for row in rows]

            # Then Result should contain timestamps <expected_values>
            assert_datetime_type(result, can_be_none=can_be_none, require_tzinfo=True)
            assert to_utc(result) == expected_values

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_timestamp_ltz(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
        table_name = f"{tmp_schema}.large_timestamp_ltz_table"
        execute_query(f"CREATE TABLE {table_name} (col TIMESTAMP_LTZ)")
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
        assert_datetime_type(values, require_tzinfo=True)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp, compare=compare_ts_utc)


@with_paramstyle("qmark")
class TestTimestampLtzBinding:
    """Tests for TIMESTAMP_LTZ type using parameter binding.

    The driver binds datetimes as TIMESTAMP_NTZ (see PYTHON_TO_SNOWFLAKE_TYPE),
    so ?::TIMESTAMP_LTZ casts NTZ->LTZ treating the wall-clock time as session-local.
    Exact UTC values depend on session timezone, so we only verify types and counts.
    """

    def test_should_select_timestamp_ltz_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
        result = execute_query(
            "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ",
            (TS_2024_JAN, TS_2024_JUN),
            single_row=True,
        )

        # Then Result should contain the bound timestamps
        assert_datetime_type(result, require_tzinfo=True)
        assert len(result) == 2

        # When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
        result = execute_query("SELECT ?::TIMESTAMP_LTZ", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    def test_should_insert_timestamp_ltz_using_parameter_binding(self, execute_query, executemany_insert, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with TIMESTAMP_LTZ column exists
        table_name = f"{tmp_schema}.timestamp_ltz_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col TIMESTAMP_LTZ)")

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
        assert_datetime_type(non_null_results, require_tzinfo=True)
