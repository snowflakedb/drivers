"""TIMESTAMP_NTZ type tests for Universal Driver -- pandas consumer.

Mirrors ``tests/definitions/shared/types/timestamp_ntz.feature`` and
``python/tests/e2e/types/test_timestamp_ntz.py`` using
``cursor.fetch_pandas_all()`` / ``cursor.fetch_pandas_batches()``.

Arrow timestamp -> pandas ``datetime64[ns]`` with naive ``pd.Timestamp`` cells.
NULL maps to ``pd.NaT``.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pandas as pd
import pytest

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_datetime_type,
    assert_dtypes,
    assert_timezone,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_datetime64,
    is_datetime64_tz,
)
from tests.e2e.types.utils import assert_sequential_values, batch_insert


# Constants aligned with python/tests/e2e/types/test_timestamp_ntz.py
TS_2024_JAN = datetime(2024, 1, 15, 10, 30, 0)
TS_2024_JUN = datetime(2024, 6, 20, 14, 45, 30)
TS_EPOCH = datetime(1970, 1, 1, 0, 0, 0)
TS_WITH_MICROSECONDS = datetime(2024, 1, 15, 10, 30, 0, 123456)

TS_2024_JAN_STR = "2024-01-15 10:30:00"
TS_2024_JUN_STR = "2024-06-20 14:45:30"
TS_EPOCH_STR = "1970-01-01 00:00:00"
TS_WITH_MICROSECONDS_STR = "2024-01-15 10:30:00.123456"

LARGE_RESULT_SET_SIZE = 50_000

LITERAL_PARAM_CASES = [
    ([TS_2024_JAN_STR, TS_2024_JUN_STR], (TS_2024_JAN, TS_2024_JUN)),
    ([TS_EPOCH_STR], (TS_EPOCH,)),
    ([TS_WITH_MICROSECONDS_STR], (TS_WITH_MICROSECONDS,)),
]

TABLE_SELECT_TEST_CASES = [
    ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN]),
    ("epoch", [TS_EPOCH_STR, TS_2024_JAN_STR], [TS_EPOCH, TS_2024_JAN]),
    ("microseconds", [TS_2024_JAN_STR, TS_WITH_MICROSECONDS_STR], [TS_2024_JAN, TS_WITH_MICROSECONDS]),
    ("null", [None, TS_2024_JAN_STR], [TS_2024_JAN, pd.NaT]),
]

SEQUENTIAL_TIMESTAMP_BASE = datetime(2024, 1, 1, 0, 0, 0)


def sequential_timestamp(i):
    return SEQUENTIAL_TIMESTAMP_BASE + timedelta(seconds=i)


class TestFetchPandasTimestampNtzTypeCasting:
    """Type-casting coverage for TIMESTAMP_NTZ via fetch_pandas_all."""

    def test_should_cast_timestamp_ntz_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ" is executed
        df = execute_and_fetch(cursor, f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_NTZ")

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_datetime64])
        val = get_row(df, 0)[0]
        assert isinstance(val, pd.Timestamp)
        assert_datetime_type((val,))
        assert val == TS_2024_JAN
        # And Values should not have timezone info
        assert_timezone((val,), expected_tz=None)


class TestFetchPandasTimestampNtzLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    @pytest.mark.parametrize(
        "query_values,expected_values",
        LITERAL_PARAM_CASES,
    )
    def test_should_select_timestamp_ntz_values(self, cursor, query_values, expected_values):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"'{v}'::TIMESTAMP_NTZ" for v in query_values)
        df = execute_and_fetch(cursor, f"SELECT {select_cols}")

        # Then Result should contain timestamps <expected_values>
        row = get_row(df, 0)
        assert_dtypes(df, [is_datetime64 for _ in expected_values])
        assert tuple(row) == expected_values
        # And Values should not have timezone info
        assert_datetime_type(row)
        assert_timezone(row, expected_tz=None)

    def test_should_handle_null_values_for_timestamp_ntz(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ",
        )

        # Then Result should contain [2024-01-15 10:30:00, NULL]
        assert_dtypes(df, [is_datetime64, is_datetime64])
        row = get_row(df, 0)
        assert_datetime_type((row[0],))
        assert_timezone((row[0],), expected_tz=None)
        assert row[0] == TS_2024_JAN
        assert row[1] is pd.NaT

    def test_should_download_large_result_set_with_multiple_chunks_for_timestamp_ntz(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,
        #   '2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY ts",
        )

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        col = get_column(combined, 0)
        assert_datetime_type(col)
        assert_timezone(col, expected_tz=None)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


class TestFetchPandasTimestampNtzTable:
    """Table-based scenarios via fetch_pandas_all."""

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values",
        TABLE_SELECT_TEST_CASES,
        ids=[c[0] for c in TABLE_SELECT_TEST_CASES],
    )
    def test_should_select_values_from_table_for_timestamp_ntz(
        self, execute_query, cursor, tmp_schema, values_name, insert_values, expected_values
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_NTZ column exists with values <insert_values>
        table_name = f"{tmp_schema}.pd_ts_ntz_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_NTZ)")
        batch_insert(execute_query, table_name, insert_values, quote_strings=True)

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then Result should contain timestamps <expected_values>
        col = get_column(df, 0)
        assert_dtypes(df, [is_datetime64])
        assert col == expected_values
        # And Values should not have timezone info
        assert_datetime_type(col, can_be_none=True)
        assert_timezone(col, expected_tz=None, can_be_none=True)

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_timestamp_ntz(
        self, execute_query, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_NTZ column exists with 50000 sequential timestamp values
        table_name = f"{tmp_schema}.pd_large_ts_ntz_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_NTZ)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00'::TIMESTAMP_NTZ) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        col = get_column(combined, 0)
        assert_datetime_type(col)
        assert_timezone(col, expected_tz=None)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


@with_paramstyle("qmark")
class TestFetchPandasTimestampNtzBinding:
    """Parameter-binding scenarios via fetch_pandas_all.

    Naive datetimes are stored as-is. Tz-aware datetimes are converted to UTC
    then stripped — only the UTC wall-clock is stored.
    """

    def test_should_select_timestamp_ntz_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ" is executed with bound timestamp values
        df = execute_and_fetch(
            cursor,
            "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ",
            params=(TS_2024_JAN, TS_2024_JUN),
        )

        # Then Result should contain [2024-01-15 10:30:00, 2024-06-20 14:45:30]
        row = get_row(df, 0)
        assert_dtypes(df, [is_datetime64, is_datetime64])
        assert tuple(row) == (TS_2024_JAN, TS_2024_JUN)
        # And Values should not have timezone info
        assert_datetime_type(row)
        assert_timezone(row, expected_tz=None)

    def test_should_return_null_when_selecting_timestamp_ntz_using_parameter_binding_with_null_value(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound NULL value
        df = execute_and_fetch(cursor, "SELECT ?::TIMESTAMP_NTZ", params=(None,))

        # Then Result should contain [NULL]
        assert_dtypes(df, [is_datetime64])
        assert get_row(df, 0)[0] is pd.NaT

    def test_should_insert_timestamp_ntz_using_parameter_binding(
        self, execute_query, executemany_insert, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_NTZ column exists
        table_name = f"{tmp_schema}.pd_ts_ntz_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_NTZ)")

        # When Timestamp values are bulk-inserted using multirow binding
        test_values = [
            (TS_2024_JAN,),
            (TS_2024_JUN,),
            (None,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then SELECT should return the inserted values in ascending order
        col = get_column(df, 0)
        assert_dtypes(df, [is_datetime64])
        assert col[0] == TS_2024_JAN
        assert col[1] == TS_2024_JUN
        assert pd.isna(col[2])
        assert_datetime_type((col[0], col[1]))
        assert_timezone((col[0], col[1]), expected_tz=None)

    @pytest.mark.parametrize(
        "aware_input,expected",
        [
            (datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone.utc), datetime(2024, 1, 15, 10, 30, 0)),
            (
                datetime(2024, 1, 15, 12, 30, 0, tzinfo=timezone(timedelta(hours=2))),
                datetime(2024, 1, 15, 10, 30, 0),
            ),
            (
                datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone(timedelta(hours=-5))),
                datetime(2024, 1, 15, 15, 30, 0),
            ),
        ],
    )
    def test_should_store_utc_equivalent_when_binding_timezone_aware_datetime_to_timestamp_ntz(
        self, cursor, aware_input, expected
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound aware datetime <input>
        df = execute_and_fetch(cursor, "SELECT ?::TIMESTAMP_NTZ", params=(aware_input,))

        # Then Result should contain [<expected>]
        assert_dtypes(df, [is_datetime64])
        val = get_row(df, 0)[0]
        assert val == expected
        # And Values should not have timezone info
        assert_timezone((val,), expected_tz=None)


class TestFetchPandasTimestampNtzAliases:
    """TIMESTAMP / DATETIME aliases controlled by TIMESTAMP_TYPE_MAPPING."""

    @pytest.mark.parametrize("type_name", ["TIMESTAMP", "DATETIME"])
    def test_should_return_naive_datetime_for_type_name_alias_when_session_mapping_is_timestamp_ntz(
        self, execute_query, cursor, type_name
    ):
        # Given Snowflake client is logged in
        pass

        try:
            # And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_NTZ
            execute_query("ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'")

            # When Query "SELECT '2024-01-15 10:30:00'::<type_name>" is executed
            df = execute_and_fetch(cursor, f"SELECT '{TS_2024_JAN_STR}'::{type_name}")

            # Then All values should be returned as appropriate type
            assert_dtypes(df, [is_datetime64])
            val = get_row(df, 0)[0]
            assert isinstance(val, pd.Timestamp)
            assert_datetime_type((val,))
            assert val == TS_2024_JAN
            # And Values should not have timezone info
            assert_timezone((val,), expected_tz=None)
        finally:
            execute_query("ALTER SESSION UNSET TIMESTAMP_TYPE_MAPPING")

    def test_should_return_aware_datetime_for_timestamp_alias_when_session_mapping_is_timestamp_ltz(
        self, execute_query, cursor
    ):
        # Given Snowflake client is logged in
        pass

        try:
            # And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_LTZ
            execute_query("ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_LTZ'")

            # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP" is executed
            df = execute_and_fetch(cursor, f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP")

            # Then All values should be returned as appropriate type
            assert_dtypes(df, [is_datetime64_tz])
            val = get_row(df, 0)[0]
            assert isinstance(val, pd.Timestamp)
            assert_datetime_type((val,))
            # And Values should have timezone info
            assert val.tzinfo is not None
        finally:
            execute_query("ALTER SESSION UNSET TIMESTAMP_TYPE_MAPPING")


class TestFetchPandasTimestampNtzPrecision:
    """Pandas ``Timestamp`` can retain nanoseconds from Arrow (vs Python ``datetime`` μs cap).

    ``python/tests/e2e/types/test_timestamp_ntz.py`` asserts microsecond truncation because
    plain ``datetime`` drops digits 7–9. Here we assert the **actual** sub-microsecond
    values surfaced through ``fetch_pandas_all()`` where supported.
    """

    @pytest.mark.parametrize(
        "input_str,expected_microsecond,expected_nanosecond",
        [
            ("2024-01-15 10:30:00.123456789", 123456, 789),
            ("2024-01-15 10:30:00.999999999", 999999, 999),
        ],
    )
    def test_should_truncate_nanosecond_precision_to_microseconds_for_timestamp_ntz(
        self, cursor, input_str, expected_microsecond, expected_nanosecond
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '<input>'::TIMESTAMP_NTZ" is executed
        df = execute_and_fetch(cursor, f"SELECT '{input_str}'::TIMESTAMP_NTZ")

        # Then Result should contain [<expected>]
        assert_dtypes(df, [is_datetime64])
        val = pd.Timestamp(get_row(df, 0)[0])
        assert val == pd.Timestamp(input_str)
        assert val.microsecond == expected_microsecond
        assert val.nanosecond == expected_nanosecond
        # And Values should not have timezone info
        assert_datetime_type((val,))
        assert_timezone((val,), expected_tz=None)
