"""Pandas fetch method tests (Python-specific).

This module tests the cursor methods that return results as Pandas DataFrames:
- fetch_pandas_all
- fetch_pandas_batches
"""

from __future__ import annotations

import json

from datetime import time
from decimal import Decimal

import pandas as pd
import pytest

from tests.compatibility import is_old_driver
from tests.e2e.types.utils import assert_connection_is_open


LARGE_RESULT_SET_ROW_COUNT = 100_000

PANDAS_TYPE_CASES = [
    ("number", "1::NUMBER", "NULL::NUMBER", lambda v: v == 1),
    ("scaled_number", "3.14::NUMBER(10,2)", "NULL::NUMBER(10,2)", lambda v: abs(v - 3.14) < 0.01),
    ("varchar", "'hello'::VARCHAR", "NULL::VARCHAR", lambda v: v == "hello"),
    ("float", "1.5::FLOAT", "NULL::FLOAT", lambda v: abs(v - 1.5) < 0.01),
    ("boolean", "TRUE::BOOLEAN", "NULL::BOOLEAN", lambda v: v),
    ("date", "'2026-03-23'::DATE", "NULL::DATE", lambda v: (v.year, v.month, v.day) == (2026, 3, 23)),
    ("time", "'12:30:00'::TIME", "NULL::TIME", lambda v: isinstance(v, time) and v == time(12, 30, 0)),
    (
        "timestamp_ntz",
        "'2026-03-23 10:30:00'::TIMESTAMP_NTZ",
        "NULL::TIMESTAMP_NTZ",
        lambda v: isinstance(v, pd.Timestamp) and (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 10, 30),
    ),
    (
        "timestamp_ltz",
        "'2026-03-23 10:30:00'::TIMESTAMP_LTZ",
        "NULL::TIMESTAMP_LTZ",
        lambda v: isinstance(v, pd.Timestamp) and (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 10, 30),
    ),
    (
        "timestamp_tz",
        "'2026-03-23 10:30:00 +0530'::TIMESTAMP_TZ",
        "NULL::TIMESTAMP_TZ",
        lambda v: isinstance(v, pd.Timestamp) and (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 5, 0),
    ),
    ("binary", "TO_BINARY('ABCD','HEX')::BINARY", "NULL::BINARY", lambda v: v == b"\xab\xcd"),
    ("variant", "TO_VARIANT(42)", "NULL::VARIANT", lambda v: json.loads(v) == 42),
    ("array", "ARRAY_CONSTRUCT(1,2,3)::ARRAY", "NULL::ARRAY", lambda v: json.loads(v) == [1, 2, 3]),
    ("object", "OBJECT_CONSTRUCT('key','value')::OBJECT", "NULL::OBJECT", lambda v: json.loads(v) == {"key": "value"}),
]


@pytest.mark.skip_universal("SNOW-3243341 - not implemented yet")
class TestFetchPandasAll:
    """Tests for fetch_pandas_all cursor method."""

    @pytest.mark.parametrize(
        "type_name,value_expr,null_expr,check",
        PANDAS_TYPE_CASES,
        ids=[c[0] for c in PANDAS_TYPE_CASES],
    )
    def test_should_fetch_type_name_with_null_as_pandas_dataframe(
        self, execute_query, cursor, type_name, value_expr, null_expr, check
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Query "ALTER SESSION SET TIMEZONE = 'UTC'" is executed
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")

        # When Query "SELECT <value_expr> AS val, <null_expr> AS null_val" is executed
        cursor.execute(f"SELECT {value_expr} AS val, {null_expr} AS null_val")

        # And fetch_pandas_all is called
        result = cursor.fetch_pandas_all()

        # Then The result should be a pandas.DataFrame with 1 row
        assert isinstance(result, pd.DataFrame)
        assert len(result) == 1

        # And Column NULL_VAL should be null
        null_val = result["NULL_VAL"].iloc[0]
        assert null_val is None or pd.isna(null_val)

        # And Column VAL should have the correct value for <type_name>
        assert check(result["VAL"].iloc[0])

    def test_should_return_empty_pandas_dataframe_for_empty_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT 1 AS id WHERE 1=0" is executed
        cursor.execute("SELECT 1 AS id WHERE 1=0")

        # And fetch_pandas_all is called
        result: pd.DataFrame = cursor.fetch_pandas_all()

        # Then The result should be a pandas.DataFrame with 0 rows
        assert isinstance(result, pd.DataFrame)
        assert len(result) == 0

    def test_should_convert_scaled_fixed_number_to_decimal_via_fetch_pandas_all(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And arrow_number_to_decimal is set to True on the connection
        if is_old_driver():
            cursor.connection.arrow_number_to_decimal_setter = True
        else:
            cursor.connection.arrow_number_to_decimal = True

        # When Query "SELECT 3.14::NUMBER(10,2) AS pi" is executed
        cursor.execute("SELECT 3.14::NUMBER(10,2) AS pi")

        # And fetch_pandas_all is called
        result = cursor.fetch_pandas_all()

        # Then Column PI should be a Python Decimal
        assert isinstance(result["PI"].iloc[0], Decimal)

    def test_should_force_microsecond_precision_for_timestamps_via_fetch_pandas_all(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ(9) AS ts" is executed
        cursor.execute("SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ(9) AS ts")

        # And fetch_pandas_all is called with force_microsecond_precision=True
        result = cursor.fetch_pandas_all(force_microsecond_precision=True)

        # Then Column TS should be a pandas Timestamp
        ts_val = result["TS"].iloc[0]
        assert isinstance(ts_val, pd.Timestamp)

        # And Column TS value should have microsecond=123456
        assert ts_val.year == 2024
        assert ts_val.month == 1
        assert ts_val.day == 15
        assert ts_val.hour == 10
        assert ts_val.minute == 30
        assert ts_val.microsecond == 123456


@pytest.mark.skip_universal("SNOW-3243341 - not implemented yet")
class TestFetchPandasBatches:
    """Tests for fetch_pandas_batches cursor method."""

    def test_should_yield_multiple_pandas_dataframes_for_large_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And fetch_pandas_batches is called
        batch_count = 0
        total_rows = 0
        for batch in cursor.fetch_pandas_batches():
            # And Each element should be a pandas.DataFrame
            assert isinstance(batch, pd.DataFrame)
            batch_count += 1
            total_rows += len(batch)

        # Then More than one DataFrame should be yielded
        assert batch_count > 1

        # And The total row count across all DataFrames should be 100000
        assert total_rows == LARGE_RESULT_SET_ROW_COUNT
