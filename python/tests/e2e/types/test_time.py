"""TIME type tests for Universal Driver.

Snowflake TIME stores wallclock time in the form HH:MI:SS with optional
fractional seconds. Precision ranges from 0 (seconds) to 9 (nanoseconds),
default is 9. Valid range: 00:00:00 to 23:59:59.999999999.
No timezone handling — all operations ignore time zones.
Python type: datetime.time (microsecond precision, digits 7–9 truncated).
"""

from __future__ import annotations

from collections import namedtuple
from datetime import time

import pytest

from ...conftest import with_paramstyle
from .utils import assert_sequential_values, assert_type, batch_insert


# =============================================================================
# EXPECTED TIME VALUES WITH STRING REPRESENTATIONS
# =============================================================================
TimeValue = namedtuple("TimeValue", ["time", "string"])
TIME_MORNING = TimeValue(time(10, 30, 0), "10:30:00")
TIME_AFTERNOON = TimeValue(time(14, 45, 30), "14:45:30")
TIME_END_OF_DAY = TimeValue(time(23, 59, 59), "23:59:59")
TIME_MIDNIGHT = TimeValue(time(0, 0, 0), "00:00:00")
TIME_NOON = TimeValue(time(12, 0, 0), "12:00:00")
TIME_WITH_MICROSECONDS = TimeValue(time(10, 30, 0, 123456), "10:30:00.123456")
TIME_FRAC = TimeValue(time(14, 45, 30, 654321), "14:45:30.654321")

# =============================================================================
# LARGE RESULT SET
# =============================================================================
LARGE_RESULT_SET_SIZE = 100_000


def _millis_to_time(ms: int) -> time:
    """Convert an integer number of milliseconds since midnight to a time object."""
    seconds, millis = divmod(ms, 1000)
    minutes, secs = divmod(seconds, 60)
    hours, mins = divmod(minutes, 60)
    return time(hours, mins, secs, millis * 1000)


class TestTimeTypeCasting:
    """Tests for TIME type casting to appropriate type."""

    def test_should_cast_time_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
        sql = f"SELECT '{TIME_MORNING.string}'::TIME, '{TIME_MIDNIGHT.string}'::TIME, '{TIME_END_OF_DAY.string}'::TIME"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, time)


class TestTimeLiteral:
    """Tests for TIME type using SELECT with literals (no tables)."""

    @pytest.mark.parametrize(
        "query_values,expected_values",
        [
            (
                [TIME_MORNING, TIME_AFTERNOON, TIME_END_OF_DAY],
                (TIME_MORNING.time, TIME_AFTERNOON.time, TIME_END_OF_DAY.time),
            ),
            (
                [TIME_MIDNIGHT],
                (TIME_MIDNIGHT.time,),
            ),
            (
                [TIME_WITH_MICROSECONDS],
                (TIME_WITH_MICROSECONDS.time,),
            ),
        ],
    )
    def test_should_select_time_values(self, execute_query, query_values, expected_values):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"'{tv.string}'::TIME" for tv in query_values)
        result = execute_query(f"SELECT {select_cols}", single_row=True)

        # Then Result should contain times <expected_values>
        assert tuple(result) == expected_values
        assert_type(result, time)

    @pytest.mark.parametrize(
        "scale,expected",
        [
            (0, time(10, 30, 0)),
            (3, time(10, 30, 0, 123000)),
            (6, time(10, 30, 0, 123456)),
        ],
    )
    def test_should_handle_time_precision_scale(self, execute_query, scale, expected):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '10:30:00.123456789'::TIME(<scale>)" is executed
        result = execute_query(
            f"SELECT '10:30:00.123456789'::TIME({scale})",
            single_row=True,
        )

        # Then Result should contain [<expected>]
        assert result[0] == expected
        assert_type(result, time)

    def test_should_handle_null_values_for_time(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
        result = execute_query(
            f"SELECT '{TIME_MORNING.string}'::TIME, NULL::TIME, '{TIME_END_OF_DAY.string}'::TIME",
            single_row=True,
        )

        # Then Result should contain [10:30:00, NULL, 23:59:59]
        assert tuple(result) == (TIME_MORNING.time, None, TIME_END_OF_DAY.time)
        assert_type(result, time, can_be_none=True)

    def test_should_download_large_result_set_with_multiple_chunks_for_time(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t
        # FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t" is executed
        sql = (
            f"SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '{TIME_MIDNIGHT.string}'::TIME) as t"
            f" FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY 1"
        )
        rows = execute_query(sql)

        # Then Result should contain 100000 sequentially increasing time values from 00:00:00
        values = [row[0] for row in rows]
        assert_type(values, time)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=_millis_to_time)


class TestTimeTable:
    """Tests for TIME type using table operations."""

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values,can_be_none",
        [
            (
                "basic",
                [TIME_MORNING, TIME_AFTERNOON, TIME_END_OF_DAY],
                [TIME_MORNING.time, TIME_AFTERNOON.time, TIME_END_OF_DAY.time],
                False,
            ),
            (
                "midnight",
                [TIME_MIDNIGHT, TIME_NOON, TIME_END_OF_DAY],
                [TIME_MIDNIGHT.time, TIME_NOON.time, TIME_END_OF_DAY.time],
                False,
            ),
            (
                "microseconds",
                [TIME_MORNING, TIME_WITH_MICROSECONDS],
                [TIME_MORNING.time, TIME_WITH_MICROSECONDS.time],
                False,
            ),
            (
                "null",
                [None, TIME_MORNING],
                [TIME_MORNING.time, None],
                True,
            ),
        ],
    )
    def test_should_select_values_from_table_for_time(
        self, execute_query, tmp_schema, values_name, insert_values, expected_values, can_be_none
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists with values <insert_values>
        table_name = f"{tmp_schema}.time_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")
        batch_insert(
            execute_query,
            table_name,
            [tv.string if tv is not None else None for tv in insert_values],
            quote_strings=True,
        )

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")
        result = [row[0] for row in rows]

        # Then Result should contain times <expected_values>
        assert result == expected_values
        assert_type(result, time, can_be_none=can_be_none)

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_time(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists with 100000 sequential time values starting from 00:00:00
        table_name = f"{tmp_schema}.large_time_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '{TIME_MIDNIGHT.string}'::TIME) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 100000 sequentially increasing time values from 00:00:00
        values = [row[0] for row in rows]
        assert_type(values, time)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=_millis_to_time)


@with_paramstyle("qmark")
class TestTimeBinding:
    """Tests for TIME type using parameter binding."""

    def test_should_select_time_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIME, ?::TIME, ?::TIME" is executed
        # with bound time values [10:30:00, 14:45:30, 23:59:59]
        result = execute_query(
            "SELECT ?::TIME, ?::TIME, ?::TIME",
            (TIME_MORNING.time, TIME_AFTERNOON.time, TIME_END_OF_DAY.time),
            single_row=True,
        )

        # Then Result should contain times [10:30:00, 14:45:30, 23:59:59]
        assert tuple(result) == (TIME_MORNING.time, TIME_AFTERNOON.time, TIME_END_OF_DAY.time)
        assert_type(result, time)

    def test_should_select_null_time_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIME" is executed with bound NULL value
        result = execute_query("SELECT ?::TIME", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    def test_should_insert_time_using_parameter_binding(self, execute_query, executemany_insert, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists
        table_name = f"{tmp_schema}.time_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")

        # When Time values [00:00:00, 10:30:00, 14:45:30, 23:59:59] are inserted using binding
        test_values = [
            (TIME_MIDNIGHT.time,),
            (TIME_MORNING.time,),
            (TIME_AFTERNOON.time,),
            (TIME_END_OF_DAY.time,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain times [00:00:00, 10:30:00, 14:45:30, 23:59:59]
        result = [row[0] for row in rows]
        assert result == [TIME_MIDNIGHT.time, TIME_MORNING.time, TIME_AFTERNOON.time, TIME_END_OF_DAY.time]
        assert_type(result, time)

    def test_should_insert_time_with_fractional_seconds_using_parameter_binding(
        self, execute_query, executemany_insert, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists
        table_name = f"{tmp_schema}.time_frac_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")

        # When Time values [10:30:00.123456, 14:45:30.654321] are bulk-inserted using multirow binding
        test_values = [(TIME_WITH_MICROSECONDS.time,), (TIME_FRAC.time,)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain times [10:30:00.123456, 14:45:30.654321]
        result = [row[0] for row in rows]
        assert result == [TIME_WITH_MICROSECONDS.time, TIME_FRAC.time]
        assert_type(result, time)
