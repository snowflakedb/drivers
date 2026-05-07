"""TIME type tests for Universal Driver -- pandas consumer.

Mirrors every scenario in ``tests/definitions/shared/types/time.feature``
using ``cursor.fetch_pandas_all()`` / ``cursor.fetch_pandas_batches()``.

Arrow time64 -> pandas object dtype. Values are ``datetime.time`` objects.
NULL -> ``None`` (object-dtype columns).
"""

from __future__ import annotations

from datetime import time

import pytest

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_object,
)
from tests.e2e.types.utils import assert_sequential_values, millis_to_time


# Test constants ported from tests/e2e/types/test_time.py
TIME_MORNING = time(10, 30, 0)
TIME_AFTERNOON = time(14, 45, 30)
TIME_END_OF_DAY = time(23, 59, 59)
TIME_MIDNIGHT = time(0, 0, 0)
TIME_NOON = time(12, 0, 0)
TIME_WITH_MICROSECONDS = time(10, 30, 0, 123456)
TIME_FRAC = time(14, 45, 30, 654321)
LARGE_RESULT_SET_SIZE = 100_000


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


class TestFetchPandasTimeTypeCasting:
    """Type-casting coverage for TIME via fetch_pandas_all."""

    def test_should_cast_time_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
        df = execute_and_fetch(cursor, "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME")

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [TIME_MORNING, TIME_MIDNIGHT, TIME_END_OF_DAY]


class TestFetchPandasTimeLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    @pytest.mark.parametrize(
        "query_values,expected_values",
        [
            (
                "'10:30:00'::TIME, '14:45:30'::TIME, '23:59:59'::TIME",
                [TIME_MORNING, TIME_AFTERNOON, TIME_END_OF_DAY],
            ),
            (
                "'00:00:00'::TIME",
                [TIME_MIDNIGHT],
            ),
            (
                "'10:30:00.123456'::TIME",
                [TIME_WITH_MICROSECONDS],
            ),
        ],
    )
    def test_should_select_time_values(self, cursor, query_values, expected_values):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        df = execute_and_fetch(cursor, f"SELECT {query_values}")

        # Then Result should contain times <expected_values>
        assert get_row(df, 0) == expected_values

    @pytest.mark.parametrize(
        "scale,expected",
        [
            (0, time(10, 30, 0)),
            (3, time(10, 30, 0, 123000)),
            (6, time(10, 30, 0, 123456)),
        ],
    )
    def test_should_handle_time_precision_scale(self, cursor, scale, expected):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '10:30:00.123456789'::TIME(<scale>)" is executed
        df = execute_and_fetch(cursor, f"SELECT '10:30:00.123456789'::TIME({scale})")

        # Then Result should contain [<expected>]
        assert_dtypes(df, [is_object])
        assert get_row(df, 0) == [expected]

    def test_should_handle_null_values_for_time(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
        df = execute_and_fetch(cursor, "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME")

        # Then Result should contain [10:30:00, NULL, 23:59:59]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [TIME_MORNING, None, TIME_END_OF_DAY]

    def test_should_download_large_result_set_with_multiple_chunks_for_time(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1,
        #   '00:00:00'::TIME) as t FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t"
            f" FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY t",
        )

        # Then Result should contain 100000 sequentially increasing time values from 00:00:00
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=millis_to_time)


class TestFetchPandasTimeTable:
    """Table-based scenarios via fetch_pandas_all."""

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values",
        [
            (
                "basic",
                ["10:30:00", "14:45:30", "23:59:59"],
                [TIME_MORNING, TIME_AFTERNOON, TIME_END_OF_DAY],
            ),
            (
                "midnight",
                ["00:00:00", "12:00:00", "23:59:59"],
                [TIME_MIDNIGHT, TIME_NOON, TIME_END_OF_DAY],
            ),
            (
                "microseconds",
                ["10:30:00", "10:30:00.123456"],
                [TIME_MORNING, TIME_WITH_MICROSECONDS],
            ),
            (
                "null",
                [None, "10:30:00"],
                [TIME_MORNING, None],
            ),
        ],
    )
    def test_should_select_values_from_table_for_time(
        self, execute_query, cursor, tmp_schema, values_name, insert_values, expected_values
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists with values <insert_values>
        table_name = f"{tmp_schema}.pd_time_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")
        for v in insert_values:
            if v is None:
                execute_query(f"INSERT INTO {table_name} VALUES (NULL)")
            else:
                execute_query(f"INSERT INTO {table_name} VALUES ('{v}')")

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then Result should contain times <expected_values>
        assert get_column(df, 0) == expected_values

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_time(
        self, execute_query, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists with 100000 sequential time values starting from 00:00:00
        table_name = f"{tmp_schema}.pd_large_time_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 100000 sequentially increasing time values from 00:00:00
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=millis_to_time)


@with_paramstyle("qmark")
class TestFetchPandasTimeBinding:
    """Parameter-binding scenarios via fetch_pandas_all."""

    def test_should_select_time_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIME, ?::TIME, ?::TIME" is executed
        # with bound time values [10:30:00, 14:45:30, 23:59:59]
        df = execute_and_fetch(
            cursor,
            "SELECT ?::TIME, ?::TIME, ?::TIME",
            params=(TIME_MORNING, TIME_AFTERNOON, TIME_END_OF_DAY),
        )

        # Then Result should contain times [10:30:00, 14:45:30, 23:59:59]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [TIME_MORNING, TIME_AFTERNOON, TIME_END_OF_DAY]

    def test_should_select_null_time_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIME" is executed with bound NULL value
        df = execute_and_fetch(cursor, "SELECT ?::TIME", params=(None,))

        # Then Result should contain [NULL]
        assert_dtypes(df, [is_object])
        assert get_row(df, 0) == [None]

    def test_should_insert_time_using_parameter_binding(self, execute_query, executemany_insert, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists
        table_name = f"{tmp_schema}.pd_time_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")

        # When Time values [00:00:00, 10:30:00, 14:45:30, 23:59:59] are inserted using binding
        test_values = [
            (TIME_MIDNIGHT,),
            (TIME_MORNING,),
            (TIME_AFTERNOON,),
            (TIME_END_OF_DAY,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain times [00:00:00, 10:30:00, 14:45:30, 23:59:59]
        assert get_column(df, 0) == [TIME_MIDNIGHT, TIME_MORNING, TIME_AFTERNOON, TIME_END_OF_DAY]

    def test_should_insert_time_with_fractional_seconds_using_parameter_binding(
        self, execute_query, executemany_insert, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIME column exists
        table_name = f"{tmp_schema}.pd_time_frac_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIME)")

        # When Time values [10:30:00.123456, 14:45:30.654321] are bulk-inserted using multirow binding
        test_values = [(TIME_WITH_MICROSECONDS,), (TIME_FRAC,)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain times [10:30:00.123456, 14:45:30.654321]
        assert get_column(df, 0) == [TIME_WITH_MICROSECONDS, TIME_FRAC]
