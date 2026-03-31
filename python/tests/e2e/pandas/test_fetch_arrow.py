"""Arrow fetch method tests (Python-specific).

This module tests the cursor methods that return results as Arrow tables:
- fetch_arrow_all
- fetch_arrow_batches
- result batch pickle + to_arrow
"""

from __future__ import annotations

import json
import pickle

from datetime import time

import pyarrow as pa
import pytest

from tests.e2e.types.utils import assert_connection_is_open


LARGE_RESULT_SET_ROW_COUNT = 100_000

ARROW_TYPE_CASES = [
    # (type_name, value_expr, null_expr, value_check, empty_column_arrow_type_check)
    ("number", "1::NUMBER", "NULL::NUMBER", lambda v: v == 1, pa.types.is_int64),
    ("scaled_number", "3.14::NUMBER(10,2)", "NULL::NUMBER(10,2)", lambda v: abs(v - 3.14) < 0.01, pa.types.is_int64),
    ("varchar", "'hello'::VARCHAR", "NULL::VARCHAR", lambda v: v == "hello", pa.types.is_string),
    ("float", "1.5::FLOAT", "NULL::FLOAT", lambda v: abs(v - 1.5) < 0.01, pa.types.is_float64),
    ("boolean", "TRUE::BOOLEAN", "NULL::BOOLEAN", lambda v: v is True, pa.types.is_boolean),
    ("date", "'2026-03-23'::DATE", "NULL::DATE", lambda v: (v.year, v.month, v.day) == (2026, 3, 23), pa.types.is_date),
    ("time", "'12:30:00'::TIME", "NULL::TIME", lambda v: v == time(12, 30, 0), pa.types.is_time),
    (
        "timestamp_ntz",
        "'2026-03-23 10:30:00'::TIMESTAMP_NTZ",
        "NULL::TIMESTAMP_NTZ",
        lambda v: (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 10, 30),
        pa.types.is_timestamp,
    ),
    (
        "timestamp_ltz",
        "'2026-03-23 10:30:00'::TIMESTAMP_LTZ",
        "NULL::TIMESTAMP_LTZ",
        lambda v: (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 10, 30),
        pa.types.is_timestamp,
    ),
    (
        "timestamp_tz",
        "'2026-03-23 10:30:00 +0530'::TIMESTAMP_TZ",
        "NULL::TIMESTAMP_TZ",
        lambda v: (v.year, v.month, v.day, v.hour, v.minute) == (2026, 3, 23, 5, 0),
        pa.types.is_timestamp,
    ),
    ("binary", "TO_BINARY('ABCD','HEX')::BINARY", "NULL::BINARY", lambda v: v == b"\xab\xcd", pa.types.is_binary),
    ("variant", "TO_VARIANT(42)", "NULL::VARIANT", lambda v: json.loads(v) == 42, pa.types.is_string),
    ("array", "ARRAY_CONSTRUCT(1,2,3)::ARRAY", "NULL::ARRAY", lambda v: json.loads(v) == [1, 2, 3], pa.types.is_string),
    (
        "object",
        "OBJECT_CONSTRUCT('key','value')::OBJECT",
        "NULL::OBJECT",
        lambda v: json.loads(v) == {"key": "value"},
        pa.types.is_string,
    ),
]


class TestFetchArrowAll:
    """Tests for fetch_arrow_all cursor method."""

    @pytest.mark.parametrize(
        "type_name,value_expr,null_expr,check,_empty_column_arrow_type_check",
        ARROW_TYPE_CASES,
        ids=[c[0] for c in ARROW_TYPE_CASES],
    )
    def test_should_fetch_type_name_with_null_as_pyarrow_table(
        self, execute_query, cursor, type_name, value_expr, null_expr, check, _empty_column_arrow_type_check
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Query "ALTER SESSION SET TIMEZONE = 'UTC'" is executed
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")

        # When Query "SELECT <value_expr> AS val, <null_expr> AS null_val" is executed
        cursor.execute(f"SELECT {value_expr} AS val, {null_expr} AS null_val")

        # And fetch_arrow_all is called
        result = cursor.fetch_arrow_all()

        # Then The result should be a pyarrow.Table with 1 row
        assert isinstance(result, pa.Table)
        assert result.num_rows == 1

        # And Column NULL_VAL should be null
        assert result.column("NULL_VAL")[0].as_py() is None

        # And Column VAL should have the correct value for <type_name>
        assert check(result.column("VAL")[0].as_py())

    def test_should_return_none_from_fetch_arrow_all_for_empty_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT 1 AS id WHERE 1=0" is executed
        cursor.execute("SELECT 1 AS id WHERE 1=0")

        # And fetch_arrow_all is called
        result: pa.Table | None = cursor.fetch_arrow_all()

        # Then The result should be None
        assert result is None

    @pytest.mark.parametrize(
        "type_name,value_expr,_null_expr,_check,empty_column_arrow_type_check",
        ARROW_TYPE_CASES,
        ids=[c[0] for c in ARROW_TYPE_CASES],
    )
    def test_should_return_empty_type_name_column_with_correct_arrow_type_when_force_return_table_is_true(
        self, execute_query, cursor, type_name, value_expr, _null_expr, _check, empty_column_arrow_type_check
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT <value_expr> AS col WHERE 1=0" is executed
        cursor.execute(f"SELECT {value_expr} AS col WHERE 1=0")

        # And fetch_arrow_all is called with force_return_table=True
        result = cursor.fetch_arrow_all(force_return_table=True)

        # Then The result should be a pyarrow.Table with 0 rows
        assert isinstance(result, pa.Table)
        assert result.num_rows == 0

        # And Column COL should have <arrow_type> Arrow type
        assert empty_column_arrow_type_check(result.schema.field("COL").type)

    def test_should_convert_scaled_fixed_number_to_decimal_via_fetch_arrow_all(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And arrow_number_to_decimal is set to True on the connection
        cursor.connection.arrow_number_to_decimal_setter = True

        # When Query "SELECT 3.14::NUMBER(10,2) AS pi" is executed
        cursor.execute("SELECT 3.14::NUMBER(10,2) AS pi")

        # And fetch_arrow_all is called
        result = cursor.fetch_arrow_all()

        # Then Column PI should be Decimal128
        assert pa.types.is_decimal128(result.schema.field("PI").type)

    def test_should_force_microsecond_precision_for_timestamps_via_fetch_arrow_all(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ(9) AS ts" is executed
        cursor.execute("SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ(9) AS ts")

        # And fetch_arrow_all is called with force_microsecond_precision=True
        result = cursor.fetch_arrow_all(force_microsecond_precision=True)

        # Then Column TS should be a timestamp type with microsecond unit
        ts_type = result.schema.field("TS").type
        assert pa.types.is_timestamp(ts_type)
        assert ts_type.unit == "us"

        # And Column TS value should have microsecond=123456
        ts_val = result.column("TS")[0].as_py()
        assert ts_val.year == 2024
        assert ts_val.month == 1
        assert ts_val.day == 15
        assert ts_val.hour == 10
        assert ts_val.minute == 30
        assert ts_val.microsecond == 123456


class TestFetchArrowBatches:
    """Tests for fetch_arrow_batches cursor method."""

    def test_should_yield_multiple_arrow_batches_for_large_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And fetch_arrow_batches is called
        batch_count = 0
        total_rows = 0
        for batch in cursor.fetch_arrow_batches():
            # And Each element should be a pyarrow.Table
            assert isinstance(batch, pa.Table)
            batch_count += 1
            total_rows += batch.num_rows

        # Then More than one batch should be yielded
        assert batch_count > 1

        # And The total row count across all batches should be 100000
        assert total_rows == LARGE_RESULT_SET_ROW_COUNT


class TestResultBatchPickleArrow:
    """Tests for result batch pickle round-trip with to_arrow conversion."""

    def test_should_survive_pickle_round_trip_and_convert_to_arrow(self, execute_query, cursor, connection_factory):
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

        # And The batches are fetched via to_arrow with a fresh connection
        with connection_factory() as fresh_conn:
            # Then Fetching all deserialized batches via to_arrow should return 100000 total rows
            total_rows = sum(batch.to_arrow(connection=fresh_conn).num_rows for batch in restored_batches)
            assert total_rows == LARGE_RESULT_SET_ROW_COUNT
