"""TIMESTAMP_TZ type tests for Universal Driver.

TIMESTAMP_TZ (Time Zone) stores timestamp with the original timezone offset preserved.
Unlike TIMESTAMP_LTZ (which converts to session timezone on retrieval), TZ keeps the
exact offset that was stored. Unlike TIMESTAMP_NTZ, TZ always has tzinfo.
Python type: datetime with pytz.FixedOffset tzinfo matching the stored offset.

All SQL string literals include explicit timezone offsets to exercise offset preservation.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from ...conftest import with_paramstyle
from .utils import assert_datetime_type, assert_sequential_values, batch_insert


# =============================================================================
# EXPECTED DATETIME VALUES (with timezone offsets)
# =============================================================================
TZ_PLUS_5 = timezone(timedelta(hours=5))
TZ_MINUS_8 = timezone(timedelta(hours=-8))

TS_2024_JAN = datetime(2024, 1, 15, 10, 30, 0, tzinfo=TZ_PLUS_5)
TS_2024_JUN = datetime(2024, 6, 20, 14, 45, 30, tzinfo=TZ_MINUS_8)
TS_EPOCH = datetime(1970, 1, 1, 0, 0, 0, tzinfo=timezone.utc)
TS_WITH_MICROSECONDS = datetime(2024, 1, 15, 10, 30, 0, 123456, tzinfo=TZ_PLUS_5)

# =============================================================================
# SQL STRING REPRESENTATIONS (with explicit timezone offsets)
# =============================================================================
TS_2024_JAN_STR = "2024-01-15 10:30:00 +05:00"
TS_2024_JUN_STR = "2024-06-20 14:45:30 -08:00"
TS_EPOCH_STR = "1970-01-01 00:00:00 +00:00"
TS_WITH_MICROSECONDS_STR = "2024-01-15 10:30:00.123456 +05:00"

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


class TestTimestampTzTypeCasting:
    """Tests for TIMESTAMP_TZ type casting to appropriate type."""

    def test_should_cast_timestamp_tz_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ" is executed
        result = execute_query(f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_TZ", single_row=True)

        # Then All values should be returned as appropriate type
        assert_datetime_type(result)

        # And Values should have timezone info
        assert_datetime_type(result, require_tzinfo=True)


class TestTimestampTzLiteral:
    """Tests for TIMESTAMP_TZ type using SELECT with literals (no tables)."""

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
    def test_should_select_timestamp_tz_values(self, execute_query, values, query_values, expected_values):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"'{v}'::TIMESTAMP_TZ" for v in query_values)
        result = execute_query(f"SELECT {select_cols}", single_row=True)

        # Then Result should contain timestamps <expected_values>
        assert_datetime_type(result, require_tzinfo=True)
        assert tuple(to_utc(result)) == tuple(to_utc(expected_values))

        # And Values should have timezone info
        for actual, expected in zip(result, expected_values):
            assert actual.utcoffset() == expected.utcoffset(), (
                f"Expected offset {expected.utcoffset()}, got {actual.utcoffset()}"
            )

    def test_should_preserve_timezone_offset_for_timestamp_tz(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ" is executed
        result = execute_query(
            "SELECT '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ",
            single_row=True,
        )

        # Then Result should preserve offsets [+05:30, -08:00, +00:00, +04:30, -02:30]
        expected_offsets = [
            timedelta(hours=5, minutes=30),
            timedelta(hours=-8),
            timedelta(0),
            timedelta(hours=4, minutes=30),
            timedelta(hours=-2, minutes=-30),
        ]
        for val, expected_offset in zip(result, expected_offsets):
            actual_offset = val.utcoffset()
            assert actual_offset == expected_offset, f"Expected offset {expected_offset}, got {actual_offset}"

    EDGE_DATE_TEST_CASES = [
        ("year 9999", "9999-12-31 23:59:59 +00:00", datetime(9999, 12, 31, 23, 59, 59, tzinfo=timezone.utc)),
        ("year 1900", "1900-01-01 00:00:00 +00:00", datetime(1900, 1, 1, 0, 0, 0, tzinfo=timezone.utc)),
        ("pre-epoch", "1960-06-15 12:00:00 +05:00", datetime(1960, 6, 15, 12, 0, 0, tzinfo=TZ_PLUS_5)),
    ]

    @pytest.mark.parametrize(
        "values,query_str,expected",
        EDGE_DATE_TEST_CASES,
        ids=[c[0] for c in EDGE_DATE_TEST_CASES],
    )
    def test_should_select_edge_date_timestamp_tz_values(self, execute_query, values, query_str, expected):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        result = execute_query(f"SELECT '{query_str}'::TIMESTAMP_TZ", single_row=True)

        # Then Result should contain timestamps <expected_values>
        assert_datetime_type(result, require_tzinfo=True)
        assert result[0].astimezone(timezone.utc) == expected.astimezone(timezone.utc)

        # And Values should have timezone info
        assert result[0].utcoffset() == expected.utcoffset()

    def test_should_handle_null_values_for_timestamp_tz(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ" is executed
        result = execute_query(
            f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ",
            single_row=True,
        )

        # Then Result should contain [2024-01-15 10:30:00 +05:00, NULL]
        assert_datetime_type(result, can_be_none=True, require_tzinfo=True)
        assert to_utc(result) == [TS_2024_JAN.astimezone(timezone.utc), None]

    def test_should_download_large_result_set_with_multiple_chunks_for_timestamp_tz(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,
        #   '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        sql = (
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY 1"
        )
        rows = execute_query(sql)

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
        values = [row[0] for row in rows]
        assert_datetime_type(values, require_tzinfo=True)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp, compare=compare_ts_utc)


class TestTimestampTzTable:
    """Tests for TIMESTAMP_TZ type using table operations."""

    TABLE_SELECT_TEST_CASES = [
        ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN], False),
        ("epoch", [TS_EPOCH_STR, TS_2024_JAN_STR], [TS_EPOCH, TS_2024_JAN], False),
        ("microseconds", [TS_2024_JAN_STR, TS_WITH_MICROSECONDS_STR], [TS_2024_JAN, TS_WITH_MICROSECONDS], False),
        ("null", [None, TS_2024_JAN_STR], [TS_2024_JAN, None], True),
    ]

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values,can_be_none",
        TABLE_SELECT_TEST_CASES,
        ids=[c[0] for c in TABLE_SELECT_TEST_CASES],
    )
    def test_should_select_values_from_table_for_timestamp_tz(
        self, execute_query, tmp_schema, values_name, insert_values, expected_values, can_be_none
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_TZ column exists with values <insert_values>
        table_name = f"{tmp_schema}.timestamp_tz_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_TZ)")
        batch_insert(execute_query, table_name, insert_values, quote_strings=True)

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")
        result = [row[0] for row in rows]

        # Then Result should contain timestamps <expected_values>
        assert_datetime_type(result, can_be_none=can_be_none, require_tzinfo=True)
        assert to_utc(result) == to_utc(expected_values)

        # And Values should have timezone info
        for val in result:
            if val is not None:
                assert val.tzinfo is not None

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_timestamp_tz(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_TZ column exists with 50000 sequential timestamp values
        table_name = f"{tmp_schema}.large_timestamp_tz_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_TZ)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
        values = [row[0] for row in rows]
        assert_datetime_type(values, require_tzinfo=True)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp, compare=compare_ts_utc)


@with_paramstyle("qmark")
class TestTimestampTzBinding:
    """Tests for TIMESTAMP_TZ type using parameter binding.

    The driver binds datetimes as TIMESTAMP_NTZ (see PYTHON_TO_SNOWFLAKE_TYPE).
    For tz-aware datetimes, the driver converts to UTC and strips tzinfo before
    sending epoch nanoseconds. The ?::TIMESTAMP_TZ cast on the server then
    assigns the session timezone as the offset. Because the offset depends on
    the session timezone, we only verify types and counts — not exact offsets.
    """

    def test_should_select_timestamp_tz_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ" is executed with bound timestamp values
        result = execute_query(
            "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ",
            (TS_2024_JAN, TS_2024_JUN),
            single_row=True,
        )

        # Then Result should contain the bound timestamps
        assert_datetime_type(result, require_tzinfo=True)
        assert len(result) == 2

        # And Values should have timezone info
        for val in result:
            assert val.tzinfo is not None

    def test_should_select_null_timestamp_tz_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_TZ" is executed with bound NULL value
        result = execute_query("SELECT ?::TIMESTAMP_TZ", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    def test_should_insert_timestamp_tz_using_parameter_binding(self, execute_query, executemany_insert, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_TZ column exists
        table_name = f"{tmp_schema}.timestamp_tz_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_TZ)")

        # When Timestamp values are bulk-inserted using multirow binding
        test_values = [
            (TS_2024_JAN,),
            (TS_2024_JUN,),
            (None,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")
        result = [row[0] for row in rows]

        # Then SELECT should return the same values in any order
        non_null_results = [r for r in result if r is not None]
        null_results = [r for r in result if r is None]
        assert len(non_null_results) == 2
        assert len(null_results) == 1
        assert_datetime_type(non_null_results, require_tzinfo=True)


class TestTimestampTzPrecision:
    """Python-specific precision behaviour for TIMESTAMP_TZ.

    Python datetime is capped at microsecond precision (6 decimal places).
    Sub-microsecond digits received from Snowflake are silently truncated — not rounded.
    The .999999999 case is the critical proof: rounding would increment the second,
    truncation does not.
    """

    @pytest.mark.parametrize(
        "input_str,expected",
        [
            ("2024-01-15 10:30:00.123456789 +05:00", datetime(2024, 1, 15, 10, 30, 0, 123456, tzinfo=TZ_PLUS_5)),
            ("2024-01-15 10:30:00.999999999 +05:00", datetime(2024, 1, 15, 10, 30, 0, 999999, tzinfo=TZ_PLUS_5)),
        ],
    )
    def test_should_truncate_nanosecond_precision_to_microseconds_for_timestamp_tz(
        self, execute_query, input_str, expected
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '<input>'::TIMESTAMP_TZ" is executed
        result = execute_query(f"SELECT '{input_str}'::TIMESTAMP_TZ", single_row=True)

        # Then Result should contain [<expected>]
        assert result[0].astimezone(timezone.utc) == expected.astimezone(timezone.utc)
        assert result[0].microsecond == expected.microsecond

        # And Values should have timezone info
        assert_datetime_type(result, require_tzinfo=True)
        assert result[0].tzinfo is not None
