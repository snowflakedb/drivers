"""DATE type tests for Universal Driver.

DATE stores a calendar date (year, month, day) with no time component.
Range: 0001-01-01 to 9999-12-31. Python type: datetime.date.
"""

from __future__ import annotations

from collections import namedtuple
from datetime import date, timedelta

from ...conftest import with_paramstyle
from .utils import assert_sequential_values, assert_type, batch_insert


# =============================================================================
# EXPECTED DATE VALUES WITH STRING REPRESENTATIONS
# =============================================================================
DateValue = namedtuple("DateValue", ["date", "string"])
DATE_2024_JAN = DateValue(date(2024, 1, 15), "2024-01-15")
DATE_1999_DEC = DateValue(date(1999, 12, 31), "1999-12-31")
DATE_EPOCH = DateValue(date(1970, 1, 1), "1970-01-01")
DATE_PRE_EPOCH = DateValue(date(1969, 12, 31), "1969-12-31")
DATE_1900 = DateValue(date(1900, 1, 1), "1900-01-01")
DATE_HISTORICAL_MIN = DateValue(date(1, 1, 1), "0001-01-01")
DATE_100_MAR = DateValue(date(100, 3, 1), "0100-03-01")
DATE_GREGORIAN = DateValue(date(1582, 10, 15), "1582-10-15")
DATE_MAX = DateValue(date(9999, 12, 31), "9999-12-31")

# =============================================================================
# LARGE RESULT SET
# =============================================================================
LARGE_RESULT_SET_SIZE = 100_000
SEQUENTIAL_BASE = date(1970, 1, 1)


def sequential_date(i):
    """Transform index to expected sequential date."""
    return SEQUENTIAL_BASE + timedelta(days=i)


class TestDateTypeCasting:
    """Tests for DATE type casting to appropriate type."""

    def test_should_cast_date_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
        sql = f"SELECT '{DATE_2024_JAN.string}'::DATE, '{DATE_EPOCH.string}'::DATE, '{DATE_1999_DEC.string}'::DATE"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as DATE type
        assert_type(result, date)

        # And No precision loss should occur
        assert result == (DATE_2024_JAN.date, DATE_EPOCH.date, DATE_1999_DEC.date)


class TestDateLiteral:
    """Tests for DATE type using SELECT with literals (no tables)."""

    def test_should_select_date_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
        sql = f"SELECT '{DATE_2024_JAN.string}'::DATE, '{DATE_EPOCH.string}'::DATE, '{DATE_1999_DEC.string}'::DATE"
        result = execute_query(sql, single_row=True)

        # Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
        assert result == (DATE_2024_JAN.date, DATE_EPOCH.date, DATE_1999_DEC.date)
        assert_type(result, date)

    def test_should_select_epoch_and_pre_epoch_dates(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
        sql = f"SELECT '{DATE_EPOCH.string}'::DATE, '{DATE_PRE_EPOCH.string}'::DATE, '{DATE_1900.string}'::DATE"
        result = execute_query(sql, single_row=True)

        # Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
        assert result == (DATE_EPOCH.date, DATE_PRE_EPOCH.date, DATE_1900.date)
        assert_type(result, date)

    def test_should_select_historical_and_boundary_dates(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE" is executed
        sql = f"SELECT '{DATE_HISTORICAL_MIN.string}'::DATE, '{DATE_GREGORIAN.string}'::DATE, '{DATE_MAX.string}'::DATE"
        result = execute_query(sql, single_row=True)

        # Then Result should contain dates [0001-01-01, 1582-10-15, 9999-12-31]
        assert result == (DATE_HISTORICAL_MIN.date, DATE_GREGORIAN.date, DATE_MAX.date)
        assert_type(result, date)

    def test_should_handle_null_values_for_date(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
        sql = f"SELECT NULL::DATE, '{DATE_2024_JAN.string}'::DATE, NULL::DATE"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [NULL, 2024-01-15, NULL]
        assert result == (None, DATE_2024_JAN.date, None)
        assert_type(result, date, can_be_none=True)

    def test_should_download_large_result_set_for_date(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) as d
        # FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY d" is executed
        sql = (
            f"SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '{DATE_EPOCH.string}'::DATE) as d "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY d"
        )
        rows = execute_query(sql)

        # Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        values = [row[0] for row in rows]
        assert_type(values, date)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_date)


class TestDateTable:
    """Tests for DATE type using table operations."""

    def test_should_select_dates_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with values ['2024-01-15', '1970-01-01', '1999-12-31']
        table_name = f"{tmp_schema}.date_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        batch_insert(
            execute_query,
            table_name,
            [DATE_2024_JAN.string, DATE_EPOCH.string, DATE_1999_DEC.string],
            quote_strings=True,
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
        result = [row[0] for row in rows]

        # Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        assert result == [DATE_EPOCH.date, DATE_1999_DEC.date, DATE_2024_JAN.date]
        assert_type(result, date)

    def test_should_select_dates_with_null_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with values ['2024-01-15', NULL, '1999-12-31']
        table_name = f"{tmp_schema}.date_null_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        batch_insert(execute_query, table_name, [DATE_2024_JAN.string, None, DATE_1999_DEC.string], quote_strings=True)

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
        result = [row[0] for row in rows]

        # Then Result should contain [1999-12-31, 2024-01-15, NULL]
        assert result == [DATE_1999_DEC.date, DATE_2024_JAN.date, None]
        assert_type(result, date, can_be_none=True)

    def test_should_select_historical_and_boundary_dates_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with values ['0001-01-01', '0100-03-01', '1582-10-15', '9999-12-31']
        table_name = f"{tmp_schema}.date_historical_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        batch_insert(
            execute_query,
            table_name,
            [
                DATE_HISTORICAL_MIN.string,
                DATE_100_MAR.string,
                DATE_GREGORIAN.string,
                DATE_MAX.string,
            ],
            quote_strings=True,
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
        result = [row[0] for row in rows]

        # Then Result should contain dates [0001-01-01, 0100-03-01, 1582-10-15, 9999-12-31]
        assert result == [DATE_HISTORICAL_MIN.date, DATE_100_MAR.date, DATE_GREGORIAN.date, DATE_MAX.date]
        assert_type(result, date)

    def test_should_download_large_result_set_for_date_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with 100000 sequential dates starting from 1970-01-01
        table_name = f"{tmp_schema}.date_large_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '{DATE_EPOCH.string}'::DATE) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        values = [row[0] for row in rows]
        assert_type(values, date)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_date)


@with_paramstyle("qmark")
class TestDateBinding:
    """Tests for DATE type using parameter binding."""

    def test_should_select_date_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::DATE, ?::DATE, ?::DATE" is executed
        # with bound date values [2024-01-15, 1970-01-01, 1999-12-31]
        sql = "SELECT ?::DATE, ?::DATE, ?::DATE"
        result = execute_query(sql, (DATE_2024_JAN.date, DATE_EPOCH.date, DATE_1999_DEC.date), single_row=True)

        # Then Result should contain [2024-01-15, 1970-01-01, 1999-12-31]
        assert result == (DATE_2024_JAN.date, DATE_EPOCH.date, DATE_1999_DEC.date)
        assert_type(result, date)

    def test_should_select_null_date_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::DATE" is executed with bound NULL value
        sql = "SELECT ?::DATE"
        result = execute_query(sql, (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    def test_should_insert_date_using_parameter_binding(self, execute_query, executemany_insert, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists
        table_name = f"{tmp_schema}.date_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")

        # When Date values [2024-01-15, 1970-01-01, 1999-12-31] are inserted using parameter binding
        test_values = [
            (DATE_2024_JAN.date,),
            (DATE_EPOCH.date,),
            (DATE_1999_DEC.date,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        result = [row[0] for row in rows]
        assert result == [DATE_EPOCH.date, DATE_1999_DEC.date, DATE_2024_JAN.date]
        assert_type(result, date)
