"""INTERVAL type tests for Universal Driver.

Snowflake INTERVAL types come in two families:

YEAR TO MONTH family (YEAR, MONTH, YEAR TO MONTH):
    Stored internally as a signed count of months.
    Python type: str in Snowflake interval format:
        YEAR TO MONTH: "<sign><Y>-<MM>"  (e.g. "+1-02" for 1 year 2 months)
        YEAR:          "<sign><Y>"       (e.g. "+1" for 1 year)
        MONTH:         "<sign><M>"       (e.g. "+14" for 14 months)
    Sign is always present (+ or -). Month field is zero-padded to 2 digits.

DAY TO SECOND family (DAY, HOUR, MINUTE, SECOND, and compound forms):
    Stored internally as signed nanoseconds (int64 or Decimal128 for large values).
    Python type: timedelta (microsecond precision; nanoseconds are truncated).

IMPORTANT - timedelta range limitation:
    Python's datetime.timedelta has an asymmetric range:
        timedelta.max =  999999999 days 23:59:59.999999
        timedelta.min = -999999999 days (exactly, no sub-day component)
    Positive extreme compound intervals like '999999999 23' DAY TO HOUR fit within timedelta.max,
    but their negation '-999999999 23' requires -1000000000 days internally, which exceeds timedelta.min.
    The regular DAY TO HOUR / DAY TO MINUTE scenarios therefore use values that fit in timedelta,
    while dedicated "max literal" / "min literal" scenarios exercise full +999999999 / -999999999 spec range.
    The min scenarios assert InterfaceError (Snowflake error 252005) due to the timedelta overflow.

INTERVAL support requires ENABLE_INTERVAL_TYPE to be active on the account.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta

import pytest

from snowflake.connector import InterfaceError

from ...conftest import with_paramstyle
from .utils import assert_sequential_values, assert_type, batch_insert


# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 50_000


# =============================================================================
# TYPE CASTING
# =============================================================================


class TestIntervalTypeCasting:
    """Tests for INTERVAL type casting to appropriate Python types."""

    def test_should_cast_interval_values_to_appropriate_type_for_year_to_month_and_day_to_second(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH, '999999999-11'::INTERVAL YEAR TO MONTH,
        #   '0 0:0:1.2'::INTERVAL DAY TO SECOND, '99999 23:59:59.999999'::INTERVAL DAY TO SECOND" is executed
        sql = (
            "SELECT '1-2'::INTERVAL YEAR TO MONTH, "
            "'999999999-11'::INTERVAL YEAR TO MONTH, "
            "'0 0:0:1.2'::INTERVAL DAY TO SECOND, "
            "'99999 23:59:59.999999'::INTERVAL DAY TO SECOND"
        )
        result = execute_query(sql, single_row=True)

        # Then all INTERVAL values should be returned as appropriate type for the driver
        assert isinstance(result[0], str)
        assert isinstance(result[1], str)
        assert isinstance(result[2], timedelta)
        assert isinstance(result[3], timedelta)
        assert result[0] == "+1-02"
        assert result[1] == "+999999999-11"
        assert result[2] == timedelta(seconds=1, microseconds=200000)
        assert result[3] == timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999)


# =============================================================================
# SELECT LITERALS
# =============================================================================


class TestIntervalLiteral:
    """Tests for INTERVAL types using SELECT with literals (no tables)."""

    # ---- YEAR TO MONTH family ----

    def test_should_select_interval_year_to_month_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query selecting INTERVAL YEAR TO MONTH literals is executed
        sql = (
            "SELECT '0-0'::INTERVAL YEAR TO MONTH, "
            "'1-2'::INTERVAL YEAR TO MONTH, "
            "'-1-3'::INTERVAL YEAR TO MONTH, "
            "'999999999-11'::INTERVAL YEAR TO MONTH, "
            "'-999999999-11'::INTERVAL YEAR TO MONTH"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL YEAR TO MONTH literal values in order
        assert_type(result, str)
        assert result == ("+0-00", "+1-02", "-1-03", "+999999999-11", "-999999999-11")

    def test_should_select_interval_year_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0'::INTERVAL YEAR, '1'::INTERVAL YEAR, '-1'::INTERVAL YEAR,
        #   '999999999'::INTERVAL YEAR, '-999999999'::INTERVAL YEAR" is executed
        sql = (
            "SELECT '0'::INTERVAL YEAR, '1'::INTERVAL YEAR, '-1'::INTERVAL YEAR, "
            "'999999999'::INTERVAL YEAR, '-999999999'::INTERVAL YEAR"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL YEAR literal values in order
        assert_type(result, str)
        assert result == ("+0", "+1", "-1", "+999999999", "-999999999")

    def test_should_select_interval_month_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0'::INTERVAL MONTH, '1'::INTERVAL MONTH, '-1'::INTERVAL MONTH,
        #   '999999999'::INTERVAL MONTH, '-999999999'::INTERVAL MONTH" is executed
        sql = (
            "SELECT '0'::INTERVAL MONTH, '1'::INTERVAL MONTH, '-1'::INTERVAL MONTH, "
            "'999999999'::INTERVAL MONTH, '-999999999'::INTERVAL MONTH"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL MONTH literal values in order
        assert_type(result, str)
        assert result == ("+0", "+1", "-1", "+999999999", "-999999999")

    # ---- DAY TO SECOND family ----

    def test_should_select_interval_day_to_second_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query selecting INTERVAL DAY TO SECOND literals is executed
        sql = (
            "SELECT '0 0:0:0.0'::INTERVAL DAY TO SECOND, "
            "'12 3:4:5.678'::INTERVAL DAY TO SECOND, "
            "'-1 2:3:4.567'::INTERVAL DAY TO SECOND, "
            "'99999 23:59:59.999999'::INTERVAL DAY TO SECOND, "
            "'-99999 23:59:59.999999'::INTERVAL DAY TO SECOND"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL DAY TO SECOND literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(days=12, hours=3, minutes=4, seconds=5, microseconds=678000),
            -timedelta(days=1, hours=2, minutes=3, seconds=4, microseconds=567000),
            timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999),
            -timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999),
        )

    def test_should_select_interval_day_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0'::INTERVAL DAY, '1'::INTERVAL DAY, '-1'::INTERVAL DAY,
        #   '999999999'::INTERVAL DAY, '-999999999'::INTERVAL DAY" is executed
        sql = (
            "SELECT '0'::INTERVAL DAY, '1'::INTERVAL DAY, '-1'::INTERVAL DAY, "
            "'999999999'::INTERVAL DAY, '-999999999'::INTERVAL DAY"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL DAY literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(days=1),
            timedelta(days=-1),
            timedelta(days=999999999),
            timedelta(days=-999999999),
        )

    def test_should_select_interval_hour_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0'::INTERVAL HOUR, '1'::INTERVAL HOUR, '-1'::INTERVAL HOUR,
        #   '999999999'::INTERVAL HOUR, '-999999999'::INTERVAL HOUR" is executed
        sql = (
            "SELECT '0'::INTERVAL HOUR, '1'::INTERVAL HOUR, '-1'::INTERVAL HOUR, "
            "'999999999'::INTERVAL HOUR, '-999999999'::INTERVAL HOUR"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL HOUR literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(hours=1),
            -timedelta(hours=1),
            timedelta(hours=999999999),
            -timedelta(hours=999999999),
        )

    def test_should_select_interval_minute_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0'::INTERVAL MINUTE, '1'::INTERVAL MINUTE, '-1'::INTERVAL MINUTE,
        #   '999999999'::INTERVAL MINUTE, '-999999999'::INTERVAL MINUTE" is executed
        sql = (
            "SELECT '0'::INTERVAL MINUTE, '1'::INTERVAL MINUTE, '-1'::INTERVAL MINUTE, "
            "'999999999'::INTERVAL MINUTE, '-999999999'::INTERVAL MINUTE"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL MINUTE literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(minutes=1),
            -timedelta(minutes=1),
            timedelta(minutes=999999999),
            -timedelta(minutes=999999999),
        )

    def test_should_select_interval_second_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0'::INTERVAL SECOND, '1.0'::INTERVAL SECOND, '-1.0'::INTERVAL SECOND,
        #   '999999999.999999'::INTERVAL SECOND, '-999999999.999999'::INTERVAL SECOND" is executed
        sql = (
            "SELECT '0'::INTERVAL SECOND, '1.0'::INTERVAL SECOND, '-1.0'::INTERVAL SECOND, "
            "'999999999.999999'::INTERVAL SECOND, '-999999999.999999'::INTERVAL SECOND"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL SECOND literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(seconds=1),
            -timedelta(seconds=1),
            timedelta(seconds=999999999, microseconds=999999),
            -timedelta(seconds=999999999, microseconds=999999),
        )

    def test_should_select_interval_day_to_hour_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0 0'::INTERVAL DAY TO HOUR, '1 2'::INTERVAL DAY TO HOUR,
        #   '-1 2'::INTERVAL DAY TO HOUR" is executed
        sql = "SELECT '0 0'::INTERVAL DAY TO HOUR, '1 2'::INTERVAL DAY TO HOUR, '-1 2'::INTERVAL DAY TO HOUR"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL DAY TO HOUR literal values in order
        assert_type(result, timedelta)
        assert result == (timedelta(0), timedelta(days=1, hours=2), -timedelta(days=1, hours=2))

    def test_should_select_interval_day_to_hour_max_literal(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '999999999 23'::INTERVAL DAY TO HOUR" is executed
        sql = "SELECT '999999999 23'::INTERVAL DAY TO HOUR"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL DAY TO HOUR max value
        assert_type(result, timedelta)
        assert result == (timedelta(days=999999999, hours=23),)

    def test_should_select_interval_day_to_hour_min_literal(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '-999999999 23'::INTERVAL DAY TO HOUR" is executed
        sql = "SELECT '-999999999 23'::INTERVAL DAY TO HOUR"

        # Then the result should contain expected INTERVAL DAY TO HOUR min value
        with pytest.raises(InterfaceError, match="252005"):
            execute_query(sql, single_row=True)  # '-999999999 23' overflows timedelta.min (see module docstring).

    def test_should_select_interval_day_to_minute_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0 0:0'::INTERVAL DAY TO MINUTE, '1 2:30'::INTERVAL DAY TO MINUTE,
        #   '-1 2:30'::INTERVAL DAY TO MINUTE" is executed
        sql = (
            "SELECT '0 0:0'::INTERVAL DAY TO MINUTE, '1 2:30'::INTERVAL DAY TO MINUTE, "
            "'-1 2:30'::INTERVAL DAY TO MINUTE"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL DAY TO MINUTE literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(days=1, hours=2, minutes=30),
            -timedelta(days=1, hours=2, minutes=30),
        )

    def test_should_select_interval_day_to_minute_max_literal(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
        sql = "SELECT '999999999 23:59'::INTERVAL DAY TO MINUTE"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL DAY TO MINUTE max value
        assert_type(result, timedelta)
        assert result == (timedelta(days=999999999, hours=23, minutes=59),)

    def test_should_select_interval_day_to_minute_min_literal(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '-999999999 23:59'::INTERVAL DAY TO MINUTE" is executed
        sql = "SELECT '-999999999 23:59'::INTERVAL DAY TO MINUTE"

        # Then the result should contain expected INTERVAL DAY TO MINUTE min value
        with pytest.raises(InterfaceError, match="252005"):
            execute_query(sql, single_row=True)  # '-999999999 23:59' overflows timedelta.min (see module docstring).

    def test_should_select_interval_hour_to_minute_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0:0'::INTERVAL HOUR TO MINUTE, '1:30'::INTERVAL HOUR TO MINUTE,
        #   '-1:30'::INTERVAL HOUR TO MINUTE,
        #   '999999999:59'::INTERVAL HOUR TO MINUTE, '-999999999:59'::INTERVAL HOUR TO MINUTE" is executed
        sql = (
            "SELECT '0:0'::INTERVAL HOUR TO MINUTE, '1:30'::INTERVAL HOUR TO MINUTE, "
            "'-1:30'::INTERVAL HOUR TO MINUTE, "
            "'999999999:59'::INTERVAL HOUR TO MINUTE, '-999999999:59'::INTERVAL HOUR TO MINUTE"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL HOUR TO MINUTE literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(hours=1, minutes=30),
            -timedelta(hours=1, minutes=30),
            timedelta(hours=999999999, minutes=59),
            -timedelta(hours=999999999, minutes=59),
        )

    def test_should_select_interval_hour_to_second_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0:0:0.0'::INTERVAL HOUR TO SECOND, '1:30:45.123'::INTERVAL HOUR TO SECOND,
        #   '-1:30:45.123'::INTERVAL HOUR TO SECOND,
        #   '999999999:59:59.999999'::INTERVAL HOUR TO SECOND,
        #   '-999999999:59:59.999999'::INTERVAL HOUR TO SECOND" is executed
        sql = (
            "SELECT '0:0:0.0'::INTERVAL HOUR TO SECOND, "
            "'1:30:45.123'::INTERVAL HOUR TO SECOND, "
            "'-1:30:45.123'::INTERVAL HOUR TO SECOND, "
            "'999999999:59:59.999999'::INTERVAL HOUR TO SECOND, "
            "'-999999999:59:59.999999'::INTERVAL HOUR TO SECOND"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL HOUR TO SECOND literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(hours=1, minutes=30, seconds=45, microseconds=123000),
            -timedelta(hours=1, minutes=30, seconds=45, microseconds=123000),
            timedelta(hours=999999999, minutes=59, seconds=59, microseconds=999999),
            -timedelta(hours=999999999, minutes=59, seconds=59, microseconds=999999),
        )

    def test_should_select_interval_minute_to_second_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0:0.0'::INTERVAL MINUTE TO SECOND, '30:45.123'::INTERVAL MINUTE TO SECOND,
        #   '-30:45.123'::INTERVAL MINUTE TO SECOND,
        #   '999999999:59.999999'::INTERVAL MINUTE TO SECOND,
        #   '-999999999:59.999999'::INTERVAL MINUTE TO SECOND" is executed
        sql = (
            "SELECT '0:0.0'::INTERVAL MINUTE TO SECOND, "
            "'30:45.123'::INTERVAL MINUTE TO SECOND, "
            "'-30:45.123'::INTERVAL MINUTE TO SECOND, "
            "'999999999:59.999999'::INTERVAL MINUTE TO SECOND, "
            "'-999999999:59.999999'::INTERVAL MINUTE TO SECOND"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL MINUTE TO SECOND literal values in order
        assert_type(result, timedelta)
        assert result == (
            timedelta(0),
            timedelta(minutes=30, seconds=45, microseconds=123000),
            -timedelta(minutes=30, seconds=45, microseconds=123000),
            timedelta(minutes=999999999, seconds=59, microseconds=999999),
            -timedelta(minutes=999999999, seconds=59, microseconds=999999),
        )

    # ---- NULL ----

    def test_should_select_null_interval_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND,
        #   NULL::INTERVAL YEAR, NULL::INTERVAL SECOND" is executed
        sql = (
            "SELECT NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND, "
            "NULL::INTERVAL YEAR, NULL::INTERVAL SECOND"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain:
        assert result == (None, None, None, None)

    # ---- Bare INTERVAL (treated as seconds) ----

    def test_should_treat_interval_without_explicit_part_as_seconds(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2' AS d1,
        #   '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2 seconds' AS d2" is executed
        sql = (
            "SELECT '2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2' AS d1, "
            "'2024-04-15 12:00:00'::TIMESTAMP + INTERVAL '2 seconds' AS d2"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain:
        expected = datetime(2024, 4, 15, 12, 0, 2)
        assert result[0] == expected
        assert result[1] == expected


# =============================================================================
# SELECT FROM TABLE
# =============================================================================


class TestIntervalTable:
    """Tests for INTERVAL types using table operations."""

    def test_should_select_interval_year_to_month_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with INTERVAL YEAR TO MONTH column is created
        table_name = f"{tmp_schema}.interval_ytm_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (C1 INTERVAL YEAR TO MONTH)")

        # And The table is populated with YEAR TO MONTH values including corner cases
        batch_insert(
            execute_query,
            table_name,
            ["-999999999-11", "-1-3", "0-0", "1-2", "999999999-11", None],
            quote_strings=True,
        )

        # When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY C1 NULLS LAST")
        result = [row[0] for row in rows]

        # Then the result should contain the inserted INTERVAL YEAR TO MONTH values in order
        assert_type([r for r in result if r is not None], str)
        assert result == ["-999999999-11", "-1-03", "+0-00", "+1-02", "+999999999-11", None]

    def test_should_select_interval_day_to_second_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with INTERVAL DAY TO SECOND column is created
        table_name = f"{tmp_schema}.interval_dts_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (C1 INTERVAL DAY TO SECOND)")

        # And The table is populated with DAY TO SECOND values including corner cases
        batch_insert(
            execute_query,
            table_name,
            ["0 0:0:0.0", "12 3:4:5.678", "-1 2:3:4.567", "99999 23:59:59.999999", "-99999 23:59:59.999999", None],
            quote_strings=True,
        )

        # When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY C1 NULLS LAST")
        result = [row[0] for row in rows]

        # Then the result should contain the inserted INTERVAL DAY TO SECOND values in order
        assert_type([r for r in result if r is not None], timedelta)
        assert result == [
            -timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999),
            -timedelta(days=1, hours=2, minutes=3, seconds=4, microseconds=567000),
            timedelta(0),
            timedelta(days=12, hours=3, minutes=4, seconds=5, microseconds=678000),
            timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999),
            None,
        ]

    def test_should_select_interval_year_2_to_month_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with INTERVAL YEAR(2) TO MONTH column is created
        table_name = f"{tmp_schema}.interval_y2tm_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (C1 INTERVAL YEAR(2) TO MONTH)")

        # And The table is populated with values ['0-0', '1-2', '-1-3', '99-11', '-99-11', NULL]
        batch_insert(
            execute_query,
            table_name,
            ["0-0", "1-2", "-1-3", "99-11", "-99-11", None],
            quote_strings=True,
        )

        # When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY C1 NULLS LAST")
        result = [row[0] for row in rows]

        # Then the result should contain the inserted INTERVAL YEAR(2) TO MONTH values in order
        assert result == ["-99-11", "-1-03", "+0-00", "+1-02", "+99-11", None]

    def test_should_select_interval_year_7_to_month_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with INTERVAL YEAR(7) TO MONTH column is created
        table_name = f"{tmp_schema}.interval_y7tm_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (C1 INTERVAL YEAR(7) TO MONTH)")

        # And The table is populated with values ['0-0', '1-2', '-1-3', '9999999-11', '-9999999-11', NULL]
        batch_insert(
            execute_query,
            table_name,
            ["0-0", "1-2", "-1-3", "9999999-11", "-9999999-11", None],
            quote_strings=True,
        )

        # When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY C1 NULLS LAST")
        result = [row[0] for row in rows]

        # Then the result should contain the inserted INTERVAL YEAR(7) TO MONTH values in order
        assert result == ["-9999999-11", "-1-03", "+0-00", "+1-02", "+9999999-11", None]

    def test_should_select_interval_day_3_to_second_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with INTERVAL DAY(3) TO SECOND column is created
        table_name = f"{tmp_schema}.interval_d3ts_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (C1 INTERVAL DAY(3) TO SECOND)")

        # And The table is populated with values
        #   ['0 0:0:0.0', '1 2:3:4.567', '-1 2:3:4.567', '999 23:59:59.999999', '-999 23:59:59.999999', NULL]
        batch_insert(
            execute_query,
            table_name,
            ["0 0:0:0.0", "1 2:3:4.567", "-1 2:3:4.567", "999 23:59:59.999999", "-999 23:59:59.999999", None],
            quote_strings=True,
        )

        # When Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY C1 NULLS LAST")
        result = [row[0] for row in rows]

        # Then the result should contain the inserted INTERVAL DAY(3) TO SECOND values in order
        assert result == [
            -timedelta(days=999, hours=23, minutes=59, seconds=59, microseconds=999999),
            -timedelta(days=1, hours=2, minutes=3, seconds=4, microseconds=567000),
            timedelta(0),
            timedelta(days=1, hours=2, minutes=3, seconds=4, microseconds=567000),
            timedelta(days=999, hours=23, minutes=59, seconds=59, microseconds=999999),
            None,
        ]


# =============================================================================
# BINDING
# =============================================================================


@with_paramstyle("qmark")
class TestIntervalBinding:
    """Tests for INTERVAL types using parameter binding."""

    # ---- INSERT + SELECT back ----

    def test_should_insert_and_select_back_interval_year_to_month_values_using_parameter_binding(
        self, execute_query, executemany_insert, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with INTERVAL YEAR TO MONTH column is created
        table_name = f"{tmp_schema}.interval_ytm_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (C1 INTERVAL YEAR TO MONTH)")

        # When INTERVAL YEAR TO MONTH values ['0-0', '1-2', '-1-3', '999999999-11', '-999999999-11', NULL]
        #   are inserted using parameter binding
        test_values = [("0-0",), ("1-2",), ("-1-3",), ("999999999-11",), ("-999999999-11",), (None,)]

        # And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
        rows = executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)
        result = [row[0] for row in rows]

        # Then the result should contain the bound INTERVAL YEAR TO MONTH values
        #   ['-999999999-11', '-1-3', '0-0', '1-2', '999999999-11', NULL]
        assert result == ["-999999999-11", "-1-03", "+0-00", "+1-02", "+999999999-11", None]

    def test_should_insert_and_select_back_interval_day_to_second_values_using_parameter_binding(
        self, execute_query, executemany_insert, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with INTERVAL DAY TO SECOND column is created
        table_name = f"{tmp_schema}.interval_dts_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (C1 INTERVAL DAY TO SECOND)")

        # When INTERVAL DAY TO SECOND values ['0 0:0:0.0', '12 3:4:5.678', '-1 2:3:4.567',
        #   '99999 23:59:59.999999', '-99999 23:59:59.999999', NULL] are inserted using parameter binding
        test_values = [
            ("0 0:0:0.0",),
            ("12 3:4:5.678",),
            ("-1 2:3:4.567",),
            ("99999 23:59:59.999999",),
            ("-99999 23:59:59.999999",),
            (None,),
        ]

        # And Query "SELECT * FROM {table} ORDER BY C1 NULLS LAST" is executed
        rows = executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)
        result = [row[0] for row in rows]

        # Then the result should contain the bound INTERVAL DAY TO SECOND values
        #   ['-99999 23:59:59.999999', '-1 2:3:4.567', '0 0:0:0.0', '12 3:4:5.678',
        #   '99999 23:59:59.999999', NULL]
        assert result == [
            -timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999),
            -timedelta(days=1, hours=2, minutes=3, seconds=4, microseconds=567000),
            timedelta(0),
            timedelta(days=12, hours=3, minutes=4, seconds=5, microseconds=678000),
            timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999),
            None,
        ]

    # ---- SELECT with cast ----

    def test_should_select_interval_year_to_month_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH,
        #   ?::INTERVAL YEAR TO MONTH" is executed with bound string values ['0-0', '1-2', '999999999-11']
        sql = "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH, ?::INTERVAL YEAR TO MONTH"
        result = execute_query(sql, ("0-0", "1-2", "999999999-11"), single_row=True)

        # Then the result should contain:
        assert result == ("+0-00", "+1-02", "+999999999-11")

    def test_should_select_interval_day_to_second_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND,
        #   ?::INTERVAL DAY TO SECOND" is executed with bound string values
        #   ['0 0:0:0.0', '12 3:4:5.678', '99999 23:59:59.999999']
        sql = "SELECT ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND, ?::INTERVAL DAY TO SECOND"
        result = execute_query(sql, ("0 0:0:0.0", "12 3:4:5.678", "99999 23:59:59.999999"), single_row=True)

        # Then the result should contain:
        assert result == (
            timedelta(0),
            timedelta(days=12, hours=3, minutes=4, seconds=5, microseconds=678000),
            timedelta(days=99999, hours=23, minutes=59, seconds=59, microseconds=999999),
        )

    def test_should_select_null_interval_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND"
        #   is executed with bound NULL values
        sql = "SELECT ?::INTERVAL YEAR TO MONTH, ?::INTERVAL DAY TO SECOND"
        result = execute_query(sql, (None, None), single_row=True)

        # Then the result should contain:
        assert result == (None, None)

    # ---- Sub-type SELECT bindings (YEAR TO MONTH family) ----

    def test_should_select_interval_year_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL YEAR, ?::INTERVAL YEAR, ?::INTERVAL YEAR"
        #   is executed with bound string values ['0', '2', '-999999999']
        sql = "SELECT ?::INTERVAL YEAR, ?::INTERVAL YEAR, ?::INTERVAL YEAR"
        result = execute_query(sql, ("0", "2", "-999999999"), single_row=True)

        # Then the result should contain expected INTERVAL YEAR bound values in order
        assert result == ("+0", "+2", "-999999999")

    def test_should_select_interval_month_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL MONTH, ?::INTERVAL MONTH, ?::INTERVAL MONTH"
        #   is executed with bound string values ['0', '5', '-999999999']
        sql = "SELECT ?::INTERVAL MONTH, ?::INTERVAL MONTH, ?::INTERVAL MONTH"
        result = execute_query(sql, ("0", "5", "-999999999"), single_row=True)

        # Then the result should contain expected INTERVAL MONTH bound values in order
        assert result == ("+0", "+5", "-999999999")

    # ---- Sub-type SELECT bindings (DAY TO SECOND family) ----

    def test_should_select_interval_day_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY, ?::INTERVAL DAY, ?::INTERVAL DAY"
        #   is executed with bound string values ['0', '1', '-999999999']
        sql = "SELECT ?::INTERVAL DAY, ?::INTERVAL DAY, ?::INTERVAL DAY"
        result = execute_query(sql, ("0", "1", "-999999999"), single_row=True)

        # Then the result should contain expected INTERVAL DAY bound values in order
        assert result == (timedelta(0), timedelta(days=1), timedelta(days=-999999999))

    def test_should_select_interval_hour_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL HOUR, ?::INTERVAL HOUR, ?::INTERVAL HOUR"
        #   is executed with bound string values ['0', '5', '-999999999']
        sql = "SELECT ?::INTERVAL HOUR, ?::INTERVAL HOUR, ?::INTERVAL HOUR"
        result = execute_query(sql, ("0", "5", "-999999999"), single_row=True)

        # Then the result should contain expected INTERVAL HOUR bound values in order
        assert result == (timedelta(0), timedelta(hours=5), -timedelta(hours=999999999))

    def test_should_select_interval_minute_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL MINUTE, ?::INTERVAL MINUTE, ?::INTERVAL MINUTE"
        #   is executed with bound string values ['0', '4', '-999999999']
        sql = "SELECT ?::INTERVAL MINUTE, ?::INTERVAL MINUTE, ?::INTERVAL MINUTE"
        result = execute_query(sql, ("0", "4", "-999999999"), single_row=True)

        # Then the result should contain expected INTERVAL MINUTE bound values in order
        assert result == (timedelta(0), timedelta(minutes=4), -timedelta(minutes=999999999))

    def test_should_select_interval_second_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL SECOND, ?::INTERVAL SECOND, ?::INTERVAL SECOND"
        #   is executed with bound string values ['0', '8.5', '-999999999.999999']
        sql = "SELECT ?::INTERVAL SECOND, ?::INTERVAL SECOND, ?::INTERVAL SECOND"
        result = execute_query(sql, ("0", "8.5", "-999999999.999999"), single_row=True)

        # Then the result should contain expected INTERVAL SECOND bound values in order
        assert result == (
            timedelta(0),
            timedelta(seconds=8, microseconds=500000),
            -timedelta(seconds=999999999, microseconds=999999),
        )

    def test_should_select_interval_day_to_hour_value_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '1 2'
        sql = "SELECT ?::INTERVAL DAY TO HOUR"
        result = execute_query(sql, ("1 2",), single_row=True)

        # Then the result should contain expected INTERVAL DAY TO HOUR bound value
        assert result == (timedelta(days=1, hours=2),)

    def test_should_select_interval_day_to_hour_max_value_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '999999999 23'
        sql = "SELECT ?::INTERVAL DAY TO HOUR"
        result = execute_query(sql, ("999999999 23",), single_row=True)

        # Then the result should contain expected INTERVAL DAY TO HOUR max bound value
        assert result == (timedelta(days=999999999, hours=23),)

    def test_should_select_interval_day_to_hour_min_value_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY TO HOUR" is executed with bound string value '-999999999 23'
        sql = "SELECT ?::INTERVAL DAY TO HOUR"

        # Then the result should contain expected INTERVAL DAY TO HOUR min bound value
        with pytest.raises(InterfaceError, match="252005"):
            execute_query(
                sql, ("-999999999 23",), single_row=True
            )  # '-999999999 23' overflows timedelta.min (see module docstring).

    def test_should_select_interval_day_to_minute_value_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '1 2:30'
        sql = "SELECT ?::INTERVAL DAY TO MINUTE"
        result = execute_query(sql, ("1 2:30",), single_row=True)

        # Then the result should contain expected INTERVAL DAY TO MINUTE bound value
        assert result == (timedelta(days=1, hours=2, minutes=30),)

    def test_should_select_interval_day_to_minute_max_value_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '999999999 23:59'
        sql = "SELECT ?::INTERVAL DAY TO MINUTE"
        result = execute_query(sql, ("999999999 23:59",), single_row=True)

        # Then the result should contain expected INTERVAL DAY TO MINUTE max bound value
        assert result == (timedelta(days=999999999, hours=23, minutes=59),)

    def test_should_select_interval_day_to_minute_min_value_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL DAY TO MINUTE" is executed with bound string value '-999999999 23:59'
        sql = "SELECT ?::INTERVAL DAY TO MINUTE"

        # Then the result should contain expected INTERVAL DAY TO MINUTE min bound value
        with pytest.raises(InterfaceError, match="252005"):
            execute_query(
                sql, ("-999999999 23:59",), single_row=True
            )  # '-999999999 23:59' overflows timedelta.min (see module docstring).

    def test_should_select_interval_hour_to_minute_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL HOUR TO MINUTE, ?::INTERVAL HOUR TO MINUTE"
        #   is executed with bound string values ['1:30', '-999999999:59']
        sql = "SELECT ?::INTERVAL HOUR TO MINUTE, ?::INTERVAL HOUR TO MINUTE"
        result = execute_query(sql, ("1:30", "-999999999:59"), single_row=True)

        # Then the result should contain expected INTERVAL HOUR TO MINUTE bound values in order
        assert result == (
            timedelta(hours=1, minutes=30),
            -timedelta(hours=999999999, minutes=59),
        )

    def test_should_select_interval_hour_to_second_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL HOUR TO SECOND, ?::INTERVAL HOUR TO SECOND"
        #   is executed with bound string values ['1:30:45.123', '-999999999:59:59.999999']
        sql = "SELECT ?::INTERVAL HOUR TO SECOND, ?::INTERVAL HOUR TO SECOND"
        result = execute_query(sql, ("1:30:45.123", "-999999999:59:59.999999"), single_row=True)

        # Then the result should contain expected INTERVAL HOUR TO SECOND bound values in order
        assert result == (
            timedelta(hours=1, minutes=30, seconds=45, microseconds=123000),
            -timedelta(hours=999999999, minutes=59, seconds=59, microseconds=999999),
        )

    def test_should_select_interval_minute_to_second_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::INTERVAL MINUTE TO SECOND, ?::INTERVAL MINUTE TO SECOND"
        #   is executed with bound string values ['30:45.123', '-999999999:59.999999']
        sql = "SELECT ?::INTERVAL MINUTE TO SECOND, ?::INTERVAL MINUTE TO SECOND"
        result = execute_query(sql, ("30:45.123", "-999999999:59.999999"), single_row=True)

        # Then the result should contain expected INTERVAL MINUTE TO SECOND bound values in order
        assert result == (
            timedelta(minutes=30, seconds=45, microseconds=123000),
            -timedelta(minutes=999999999, seconds=59, microseconds=999999),
        )


# =============================================================================
# MULTIPLE CHUNKS DOWNLOADING
# =============================================================================


class TestIntervalMultipleChunks:
    """Tests for downloading INTERVAL data across multiple result chunks."""

    def test_should_download_interval_year_to_month_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY ym" is executed
        sql = (
            "SELECT '0-1'::INTERVAL YEAR TO MONTH * SEQ4() AS ym "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v "
            "ORDER BY ym"
        )
        rows = execute_query(sql)

        # Then there are 50000 rows returned
        values = [row[0] for row in rows]
        assert len(values) == LARGE_RESULT_SET_SIZE

        # And all returned INTERVAL YEAR TO MONTH values should form a sequential series of months starting at 0
        assert_sequential_values(
            values,
            LARGE_RESULT_SET_SIZE,
            transform=lambda m: f"+{m // 12}-{m % 12:02d}",
        )

    def test_should_download_interval_day_to_second_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY dt" is executed
        sql = (
            "SELECT '0 0:0:1.0'::INTERVAL DAY TO SECOND * SEQ4() AS dt "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v "
            "ORDER BY dt"
        )
        rows = execute_query(sql)

        # Then there are 50000 rows returned
        values = [row[0] for row in rows]
        assert len(values) == LARGE_RESULT_SET_SIZE

        # And all returned INTERVAL DAY TO SECOND values should form a sequential series of seconds starting at 0
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=lambda i: timedelta(seconds=i))


# =============================================================================
# INTERVAL ARITHMETIC
# =============================================================================


class TestIntervalArithmetic:
    """Tests for INTERVAL arithmetic operations."""

    def test_should_respect_order_of_interval_components_in_date_arithmetic(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year' AS d1,
        #   TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day' AS d2" is executed
        sql = (
            "SELECT TO_DATE('2019-02-28') + INTERVAL '1 day, 1 year' AS d1, "
            "TO_DATE('2019-02-28') + INTERVAL '1 year, 1 day' AS d2"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain:
        d1 = result[0].date() if isinstance(result[0], datetime) else result[0]
        d2 = result[1].date() if isinstance(result[1], datetime) else result[1]
        assert d1 == date(2020, 3, 1)
        assert d2 == date(2020, 2, 29)

    def test_should_support_complex_interval_with_mixed_units_and_abbreviations(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_DATE('2025-01-17') + INTERVAL '1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s,
        #   1000 ms, 445343232 us, 898498273498 ns' AS complex_interval" is executed
        sql = (
            "SELECT TO_DATE('2025-01-17') + INTERVAL "
            "'1 y, 3 q, 4 mm, 5 w, 6 d, 7 h, 9 m, 8 s, "
            "1000 ms, 445343232 us, 898498273498 ns' AS complex_interval"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain:
        expected = datetime(2027, 3, 30, 7, 31, 32, 841505)
        assert result[0] == expected

    def test_should_add_two_interval_year_to_month_values(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '1-2'::INTERVAL YEAR TO MONTH + '0-3'::INTERVAL YEAR TO MONTH AS i" is executed
        sql = "SELECT '1-2'::INTERVAL YEAR TO MONTH + '0-3'::INTERVAL YEAR TO MONTH AS i"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL YEAR TO MONTH value '1-5'
        assert result[0] == "+1-05"

    def test_should_add_two_interval_day_to_second_values(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '1 2:30:00.0'::INTERVAL DAY TO SECOND +
        #   '0 1:45:30.5'::INTERVAL DAY TO SECOND AS i" is executed
        sql = "SELECT '1 2:30:00.0'::INTERVAL DAY TO SECOND + '0 1:45:30.5'::INTERVAL DAY TO SECOND AS i"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL DAY TO SECOND value '1 4:15:30.500000'
        assert result[0] == timedelta(days=1, hours=4, minutes=15, seconds=30, microseconds=500000)

    def test_should_negate_an_interval_value(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT -('1-6'::INTERVAL YEAR TO MONTH) AS ym,
        #   -('3 12:0:0.0'::INTERVAL DAY TO SECOND) AS dt" is executed
        sql = "SELECT -('1-6'::INTERVAL YEAR TO MONTH) AS ym, -('3 12:0:0.0'::INTERVAL DAY TO SECOND) AS dt"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected negated INTERVAL values '-1-6' and '-3 12:0:0.000000'
        assert result[0] == "-1-06"
        assert result[1] == -timedelta(days=3, hours=12)

    def test_should_subtract_two_interval_values(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '1-5'::INTERVAL YEAR TO MONTH - '0-3'::INTERVAL YEAR TO MONTH AS ym,
        #   '1 4:15:30.5'::INTERVAL DAY TO SECOND - '0 1:45:30.5'::INTERVAL DAY TO SECOND AS dt" is executed
        sql = (
            "SELECT '1-5'::INTERVAL YEAR TO MONTH - '0-3'::INTERVAL YEAR TO MONTH AS ym, "
            "'1 4:15:30.5'::INTERVAL DAY TO SECOND - '0 1:45:30.5'::INTERVAL DAY TO SECOND AS dt"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL values '1-2' and '1 2:30:00.000000'
        assert result[0] == "+1-02"
        assert result[1] == timedelta(days=1, hours=2, minutes=30)

    def test_should_multiply_interval_by_a_scalar(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0-6'::INTERVAL YEAR TO MONTH * 3 AS ym,
        #   2 * '1 0:0:0.0'::INTERVAL DAY TO SECOND AS dt" is executed
        sql = "SELECT '0-6'::INTERVAL YEAR TO MONTH * 3 AS ym, 2 * '1 0:0:0.0'::INTERVAL DAY TO SECOND AS dt"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL values '1-6' and '2 0:0:0.000000'
        assert result[0] == "+1-06"
        assert result[1] == timedelta(days=2)

    def test_should_divide_interval_by_a_scalar(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '1-6'::INTERVAL YEAR TO MONTH / 3 AS ym,
        #   '2 0:0:0.0'::INTERVAL DAY TO SECOND / 2 AS dt" is executed
        sql = "SELECT '1-6'::INTERVAL YEAR TO MONTH / 3 AS ym, '2 0:0:0.0'::INTERVAL DAY TO SECOND / 2 AS dt"
        result = execute_query(sql, single_row=True)

        # Then the result should contain expected INTERVAL values '0-6' and '1 0:0:0.000000'
        assert result[0] == "+0-06"
        assert result[1] == timedelta(days=1)
