"""Pandas fetch method tests (Python-specific).

This module tests the cursor methods that return results as Pandas DataFrames:
- fetch_pandas_all
- fetch_pandas_batches
- result batch pickle + to_pandas
"""

from __future__ import annotations

import json
import pickle

from datetime import time
from decimal import Decimal

import pandas as pd
import pytest

from tests.compatibility import is_old_driver
from tests.e2e.types.utils import assert_connection_is_open


LARGE_RESULT_SET_ROW_COUNT = 100_000

PANDAS_TYPE_CASES = [
    # (type_name, value_expr, null_expr, value_check, empty_column_dtype_check)
    ("number", "1::NUMBER", "NULL::NUMBER", lambda v: v == 1, pd.api.types.is_int64_dtype),
    (
        "scaled_number",
        "3.14::NUMBER(10,2)",
        "NULL::NUMBER(10,2)",
        lambda v: abs(v - 3.14) < 0.01,
        pd.api.types.is_int64_dtype,
    ),
    ("varchar", "'hello'::VARCHAR", "NULL::VARCHAR", lambda v: v == "hello", pd.api.types.is_string_dtype),
    ("float", "1.5::FLOAT", "NULL::FLOAT", lambda v: abs(v - 1.5) < 0.01, pd.api.types.is_float_dtype),
    ("boolean", "TRUE::BOOLEAN", "NULL::BOOLEAN", lambda v: v, pd.api.types.is_bool_dtype),
    (
        "date",
        "'2026-03-23'::DATE",
        "NULL::DATE",
        lambda v: (v.year, v.month, v.day) == (2026, 3, 23),
        pd.api.types.is_object_dtype,
    ),
    (
        "time",
        "'12:30:00'::TIME",
        "NULL::TIME",
        lambda v: isinstance(v, time) and v == time(12, 30, 0),
        pd.api.types.is_object_dtype,
    ),
    (
        "timestamp_ntz",
        "'2026-03-23 10:30:00'::TIMESTAMP_NTZ",
        "NULL::TIMESTAMP_NTZ",
        lambda v: isinstance(v, pd.Timestamp) and (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 10, 30),
        pd.api.types.is_datetime64_any_dtype,
    ),
    (
        "timestamp_ltz",
        "'2026-03-23 10:30:00'::TIMESTAMP_LTZ",
        "NULL::TIMESTAMP_LTZ",
        lambda v: isinstance(v, pd.Timestamp) and (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 10, 30),
        pd.api.types.is_datetime64_any_dtype,
    ),
    (
        "timestamp_tz",
        "'2026-03-23 10:30:00 +0530'::TIMESTAMP_TZ",
        "NULL::TIMESTAMP_TZ",
        lambda v: isinstance(v, pd.Timestamp) and (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 5, 0),
        pd.api.types.is_datetime64_any_dtype,
    ),
    (
        "binary",
        "TO_BINARY('ABCD','HEX')::BINARY",
        "NULL::BINARY",
        lambda v: v == b"\xab\xcd",
        pd.api.types.is_string_dtype,
    ),
    ("variant", "TO_VARIANT(42)", "NULL::VARIANT", lambda v: json.loads(v) == 42, pd.api.types.is_string_dtype),
    (
        "array",
        "ARRAY_CONSTRUCT(1,2,3)::ARRAY",
        "NULL::ARRAY",
        lambda v: json.loads(v) == [1, 2, 3],
        pd.api.types.is_string_dtype,
    ),
    (
        "object",
        "OBJECT_CONSTRUCT('key','value')::OBJECT",
        "NULL::OBJECT",
        lambda v: json.loads(v) == {"key": "value"},
        pd.api.types.is_string_dtype,
    ),
]


class TestFetchPandasAll:
    """Tests for fetch_pandas_all cursor method."""

    @pytest.mark.parametrize(
        "type_name,value_expr,null_expr,check,_empty_column_dtype_check",
        PANDAS_TYPE_CASES,
        ids=[c[0] for c in PANDAS_TYPE_CASES],
    )
    def test_should_fetch_type_name_with_null_as_pandas_dataframe(
        self, execute_query, cursor, type_name, value_expr, null_expr, check, _empty_column_dtype_check
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

    @pytest.mark.parametrize(
        "type_name,value_expr,_null_expr,_check,empty_column_dtype_check",
        PANDAS_TYPE_CASES,
        ids=[c[0] for c in PANDAS_TYPE_CASES],
    )
    def test_should_return_empty_type_name_column_with_correct_pandas_dtype(
        self, execute_query, cursor, type_name, value_expr, _null_expr, _check, empty_column_dtype_check
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT <value_expr> AS col WHERE 1=0" is executed
        cursor.execute(f"SELECT {value_expr} AS col WHERE 1=0")

        # And fetch_pandas_all is called
        result: pd.DataFrame = cursor.fetch_pandas_all()

        # Then The result should be a pandas.DataFrame with 0 rows
        assert isinstance(result, pd.DataFrame)
        assert len(result) == 0

        # And Column COL should have <pandas_dtype> pandas dtype
        assert empty_column_dtype_check(result["COL"].dtype)

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


class TestResultBatchPicklePandas:
    """Tests for result batch pickle round-trip with to_pandas conversion."""

    def test_should_survive_pickle_round_trip_and_convert_to_pandas(self, execute_query, cursor, connection_factory):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq4() AS id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq4() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And get_result_batches is called
        batches = cursor.get_result_batches()
        assert batches is not None

        # And The batches are serialized with pickle
        pickled = pickle.dumps(batches)

        # And The batches are deserialized with pickle
        restored_batches = pickle.loads(pickled)

        # And The batches are fetched via to_pandas with a fresh connection
        with connection_factory() as fresh_conn:
            # Then Fetching all deserialized batches via to_pandas should return 100000 total rows
            total_rows = sum(len(batch.to_pandas(connection=fresh_conn)) for batch in restored_batches)
            assert total_rows == LARGE_RESULT_SET_ROW_COUNT
