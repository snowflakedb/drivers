"""TIMESTAMP_LTZ type tests for Universal Driver.

TIMESTAMP_LTZ (Local Time Zone) stores timestamp with local timezone.
Values are stored in UTC and converted to the session timezone on retrieval.
Python type: datetime with tzinfo set (not None).
"""

from __future__ import annotations

from datetime import datetime, timezone

import pytest

from .utils import assert_connection_is_open, assert_datetime_type, batch_insert

# =============================================================================
# TEST TIMESTAMPS
# =============================================================================
# Standard test timestamps
TS_2024_JAN = "2024-01-15 10:30:00"
TS_2024_JUN = "2024-06-20 14:45:30"

# Epoch timestamp (Unix epoch)
TS_EPOCH = "1970-01-01 00:00:00"

# Timestamp with microseconds
TS_WITH_MICROSECONDS = "2024-01-15 10:30:00.123456"
EXPECTED_MICROSECONDS = 123456

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 50_000


class TestTimestampLtzTypeCasting:
    """Tests for TIMESTAMP_LTZ type casting to appropriate type."""

    def test_should_cast_timestamp_ltz_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_LTZ" is executed
        result = execute_query(f"SELECT '{TS_2024_JAN}'::TIMESTAMP_LTZ", single_row=True)

        # Then All values should be returned as appropriate type
        # And Values should have timezone info
        assert_datetime_type(result, require_tzinfo=True)


class TestTimestampLtzLiteral:
    """Tests for TIMESTAMP_LTZ type using SELECT with literals (no tables)."""

    def test_should_select_timestamp_ltz_literals(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_LTZ, '2024-06-20 14:45:30'::TIMESTAMP_LTZ" is executed
        result = execute_query(
            f"SELECT '{TS_2024_JAN}'::TIMESTAMP_LTZ, '{TS_2024_JUN}'::TIMESTAMP_LTZ",
            single_row=True,
        )

        # Then Result should contain expected timestamp values
        assert_datetime_type(result, require_tzinfo=True)
        assert len(result) == 2
        # Verify the dates are correct (time may vary due to timezone conversion)
        assert result[0].year == 2024
        assert result[0].month == 1
        assert result[1].year == 2024
        assert result[1].month == 6

    def test_should_handle_null_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
        result = execute_query(
            f"SELECT '{TS_2024_JAN}'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ",
            single_row=True,
        )

        # Then Result should contain [timestamp, NULL]
        assert_datetime_type(result, can_be_none=True, require_tzinfo=True)
        assert result[1] is None

    def test_should_handle_epoch_timestamp(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '1970-01-01 00:00:00'::TIMESTAMP_LTZ" is executed
        result = execute_query(f"SELECT '{TS_EPOCH}'::TIMESTAMP_LTZ", single_row=True)

        # Then Result should contain epoch timestamp
        assert_datetime_type(result, require_tzinfo=True)
        # Convert to UTC for comparison
        utc_time = result[0].astimezone(timezone.utc)
        assert utc_time.year == 1970
        assert utc_time.month == 1
        assert utc_time.day == 1

    def test_should_handle_timestamp_with_microseconds(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00.123456'::TIMESTAMP_LTZ" is executed
        result = execute_query(f"SELECT '{TS_WITH_MICROSECONDS}'::TIMESTAMP_LTZ", single_row=True)

        # Then Result should preserve microsecond precision
        assert_datetime_type(result, require_tzinfo=True)
        assert result[0].microsecond == EXPECTED_MICROSECONDS

    def test_should_download_large_result_set_with_multiple_chunks_from_generator(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT DATEADD(second, seq8(), '2024-01-01'::TIMESTAMP_LTZ) FROM <generator>" is executed
        sql = f"SELECT DATEADD(second, seq8(), '2024-01-01'::TIMESTAMP_LTZ) FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        rows = execute_query(sql)

        # Then Result should contain expected number of timestamp values
        values = [row[0] for row in rows]
        assert_datetime_type(values, require_tzinfo=True)
        assert len(values) == LARGE_RESULT_SET_SIZE


class TestTimestampLtzTable:
    """Tests for TIMESTAMP_LTZ type using table operations."""

    def test_should_select_timestamp_ltz_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with TIMESTAMP_LTZ column exists
        table_name = f"{tmp_schema}.timestamp_ltz_table"
        execute_query(f"CREATE TABLE {table_name} (col TIMESTAMP_LTZ)")

        # And Timestamp rows are inserted
        batch_insert(execute_query, table_name, [TS_2024_JAN, TS_2024_JUN], quote_strings=True)

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain expected timestamp values
        values = [row[0] for row in rows]
        assert_datetime_type(values, require_tzinfo=True)
        assert len(values) == 2

    def test_should_handle_null_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with TIMESTAMP_LTZ column exists
        table_name = f"{tmp_schema}.null_timestamp_table"
        execute_query(f"CREATE TABLE {table_name} (col TIMESTAMP_LTZ)")

        # And Rows with NULL and non-NULL timestamps are inserted
        batch_insert(execute_query, table_name, [None, TS_2024_JAN], quote_strings=True)

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain [timestamp, NULL] in any order
        values = [row[0] for row in rows]
        assert len(values) == 2
        non_null_values = [v for v in values if v is not None]
        null_values = [v for v in values if v is None]
        assert len(non_null_values) == 1
        assert len(null_values) == 1
        assert_datetime_type(non_null_values, require_tzinfo=True)

    def test_should_download_large_result_set_with_multiple_chunks_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with TIMESTAMP_LTZ column exists with many rows
        table_name = f"{tmp_schema}.large_timestamp_table"
        execute_query(f"CREATE TABLE {table_name} (col TIMESTAMP_LTZ)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(second, seq8(), '2024-01-01'::TIMESTAMP_LTZ) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT col FROM <table>" is executed
        rows = execute_query(f"SELECT col FROM {table_name}")

        # Then Result should contain expected number of timestamp values
        values = [row[0] for row in rows]
        assert_datetime_type(values, require_tzinfo=True)
        assert len(values) == LARGE_RESULT_SET_SIZE


@pytest.mark.skip_reference
class TestTimestampLtzBinding:
    """Tests for TIMESTAMP_LTZ type using parameter binding."""

    def test_should_select_timestamp_ltz_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed
        # with bound timestamp values
        test_timestamp1 = datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone.utc)
        test_timestamp2 = datetime(2024, 6, 20, 14, 45, 30, tzinfo=timezone.utc)
        result = execute_query(
            "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ",
            (test_timestamp1, test_timestamp2),
            single_row=True,
        )

        # Then Result should contain the bound timestamps
        assert_datetime_type(result, require_tzinfo=True)

        # When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
        result = execute_query("SELECT ?::TIMESTAMP_LTZ", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    def test_should_insert_timestamp_ltz_using_parameter_binding(
        self, execute_query, executemany_insert, tmp_schema
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with TIMESTAMP_LTZ column exists
        table_name = f"{tmp_schema}.timestamp_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col TIMESTAMP_LTZ)")

        # When Timestamp values are bulk-inserted using multirow binding
        test_values = [
            (datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone.utc),),
            (datetime(2024, 6, 20, 14, 45, 30, tzinfo=timezone.utc),),
            (None,),
        ]
        rows = executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # Then SELECT should return the same values in any order
        result = [row[0] for row in rows]
        assert len(result) == 3
        non_null_results = [r for r in result if r is not None]
        assert len(non_null_results) == 2
        assert_datetime_type(non_null_results, require_tzinfo=True)
