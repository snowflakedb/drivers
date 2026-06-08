"""DATE type tests for Universal Driver -- pandas consumer.

Arrow date32 -> pandas object dtype. Values are datetime.date objects.
NULL -> None in object columns.
"""

from __future__ import annotations

from datetime import date, timedelta

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_object,
)
from tests.e2e.types.utils import assert_sequential_values


# Test constants ported from tests/e2e/types/test_date.py
DATE_2024_JAN = date(2024, 1, 15)
DATE_1999_DEC = date(1999, 12, 31)
DATE_EPOCH = date(1970, 1, 1)
DATE_PRE_EPOCH = date(1969, 12, 31)
DATE_1900 = date(1900, 1, 1)
DATE_HISTORICAL_MIN = date(1, 1, 1)
DATE_100_MAR = date(100, 3, 1)
DATE_GREGORIAN = date(1582, 10, 15)
DATE_MAX = date(9999, 12, 31)
LARGE_RESULT_SET_SIZE = 100_000
SEQUENTIAL_BASE = date(1970, 1, 1)


def sequential_date(i):
    """Transform index to expected sequential date."""
    return SEQUENTIAL_BASE + timedelta(days=i)


class TestFetchPandasDateTypeCasting:
    """Type-casting coverage for DATE via fetch_pandas_all."""

    def test_should_cast_date_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
        df = execute_and_fetch(
            cursor,
            "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE",
        )

        # Then All values should be returned as DATE type
        assert_dtypes(df, [is_object, is_object, is_object])
        # And No precision loss should occur
        assert get_row(df, 0) == [DATE_2024_JAN, DATE_EPOCH, DATE_1999_DEC]


class TestFetchPandasDateLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    def test_should_select_date_literals(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
        df = execute_and_fetch(
            cursor,
            "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE",
        )

        # Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [DATE_2024_JAN, DATE_EPOCH, DATE_1999_DEC]

    def test_should_select_epoch_and_pre_epoch_dates(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
        df = execute_and_fetch(
            cursor,
            "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE",
        )

        # Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [DATE_EPOCH, DATE_PRE_EPOCH, DATE_1900]

    def test_should_select_historical_and_boundary_dates(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE" is executed
        df = execute_and_fetch(
            cursor,
            "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE",
        )

        # Then Result should contain dates [0001-01-01, 1582-10-15, 9999-12-31]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [DATE_HISTORICAL_MIN, DATE_GREGORIAN, DATE_MAX]

    def test_should_handle_null_values_for_date(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
        df = execute_and_fetch(cursor, "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE")

        # Then Result should contain [NULL, 2024-01-15, NULL]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [None, DATE_2024_JAN, None]

    def test_should_download_large_result_set_for_date(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1,
        #   '1970-01-01'::DATE) as d FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY d" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            "SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) as d "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY d",
        )

        # Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=sequential_date)


class TestFetchPandasDateTable:
    """Table-based scenarios via fetch_pandas_all."""

    def test_should_select_dates_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with values [2024-01-15, 1970-01-01, 1999-12-31]
        table_name = f"{tmp_schema}.pd_date_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        execute_query(
            f"INSERT INTO {table_name} VALUES ('2024-01-15'::DATE), ('1970-01-01'::DATE), ('1999-12-31'::DATE)"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        assert_dtypes(df, [is_object])
        assert get_column(df, 0) == [DATE_EPOCH, DATE_1999_DEC, DATE_2024_JAN]

    def test_should_select_dates_with_null_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with values [2024-01-15, NULL, 1999-12-31]
        table_name = f"{tmp_schema}.pd_date_null_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        execute_query(f"INSERT INTO {table_name} VALUES ('2024-01-15'::DATE), (NULL), ('1999-12-31'::DATE)")

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain [1999-12-31, 2024-01-15, NULL]
        assert_dtypes(df, [is_object])
        assert get_column(df, 0) == [DATE_1999_DEC, DATE_2024_JAN, None]

    def test_should_select_historical_and_boundary_dates_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with values [0001-01-01, 0100-03-01, 1582-10-15, 9999-12-31]
        table_name = f"{tmp_schema}.pd_date_historical_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        execute_query(
            f"INSERT INTO {table_name} VALUES "
            f"('0001-01-01'::DATE), ('0100-03-01'::DATE), ('1582-10-15'::DATE), ('9999-12-31'::DATE)"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain dates [0001-01-01, 0100-03-01, 1582-10-15, 9999-12-31]
        assert_dtypes(df, [is_object])
        assert get_column(df, 0) == [DATE_HISTORICAL_MIN, DATE_100_MAR, DATE_GREGORIAN, DATE_MAX]

    def test_should_download_large_result_set_for_date_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists with 100000 sequential dates starting from 1970-01-01
        table_name = f"{tmp_schema}.pd_date_large_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(day, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '1970-01-01'::DATE) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 100000 rows with sequential dates starting from 1970-01-01
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=sequential_date)


@with_paramstyle("qmark")
class TestFetchPandasDateBinding:
    """Parameter-binding scenarios via fetch_pandas_all."""

    def test_should_select_date_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::DATE, ?::DATE, ?::DATE" is executed
        # with bound date values [2024-01-15, 1970-01-01, 1999-12-31]
        df = execute_and_fetch(
            cursor,
            "SELECT ?::DATE, ?::DATE, ?::DATE",
            params=(DATE_2024_JAN, DATE_EPOCH, DATE_1999_DEC),
        )

        # Then Result should contain [2024-01-15, 1970-01-01, 1999-12-31]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [DATE_2024_JAN, DATE_EPOCH, DATE_1999_DEC]

    def test_should_select_null_date_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::DATE" is executed with bound NULL value
        df = execute_and_fetch(cursor, "SELECT ?::DATE", params=(None,))

        # Then Result should contain [NULL]
        assert_dtypes(df, [is_object])
        assert get_row(df, 0) == [None]

    def test_should_insert_date_using_parameter_binding(self, execute_query, executemany_insert, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with DATE column exists
        table_name = f"{tmp_schema}.pd_date_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col DATE)")

        # When Date values [2024-01-15, 1970-01-01, 1999-12-31] are inserted using parameter binding
        test_data = [(DATE_2024_JAN,), (DATE_EPOCH,), (DATE_1999_DEC,)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_data)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
        assert_dtypes(df, [is_object])
        assert get_column(df, 0) == [DATE_EPOCH, DATE_1999_DEC, DATE_2024_JAN]
