"""ArrowStreamTableIterator tests.

Verifies that the ArrowStreamTableIterator correctly reads batches from an
ArrowArrayStream and applies Snowflake type conversions via ArrowTableConverter.
"""

from __future__ import annotations

from datetime import time

import pyarrow as pa

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.arrow_stream_iterator import ArrowStreamTableIterator
from tests.e2e.types.utils import assert_connection_is_open, assert_float_equal


LARGE_RESULT_SET_ROW_COUNT = 100_000


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _collect_as_table(cursor, arrow_context=None) -> pa.Table:
    """Execute the pending result set through ArrowStreamTableIterator and return a single Table."""
    stream_ptr = cursor._get_stream_ptr()
    if arrow_context is None:
        arrow_context = ArrowConverterContext()
    batches = list(ArrowStreamTableIterator(stream_ptr, arrow_context))
    assert len(batches) >= 1
    return pa.Table.from_batches(batches)


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestArrowStreamTableIterator:
    """Tests for ArrowStreamTableIterator stream consumption."""

    def test_should_yield_record_batches_for_large_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When A query producing a large result set is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And Results are collected via ArrowStreamTableIterator
        stream_ptr = cursor._get_stream_ptr()
        arrow_context = ArrowConverterContext()
        dummy_iter = ArrowStreamTableIterator(stream_ptr, arrow_context)

        # Then The iterator should yield at least one RecordBatch
        batch_count = 0
        total_rows = 0
        for batch in dummy_iter:
            assert isinstance(batch, pa.RecordBatch)
            assert batch.num_rows > 0
            batch_count += 1
            total_rows += batch.num_rows

        assert batch_count >= 1

        # And The total row count should equal the rows requested
        assert total_rows == LARGE_RESULT_SET_ROW_COUNT


class TestArrowStreamTableIteratorTypeConversions:
    """Tests that Snowflake type conversions are applied correctly."""

    def test_should_convert_scaled_fixed_number_to_double(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When A query with scaled NUMBER columns is executed
        cursor.execute(
            "SELECT 3.14::NUMBER(10,2) AS pi, -0.001::NUMBER(10,3) AS small_neg, NULL::NUMBER(10,2) AS null_num"
        )

        # And Results are collected with number_to_decimal=False (default → float64)
        table = _collect_as_table(cursor)

        # Then The PI column should be float64 with value ≈ 3.14
        assert pa.types.is_float64(table.schema.field("PI").type)
        assert_float_equal(table.column("PI")[0].as_py(), 3.14)

        # And The SMALL_NEG column should be float64 with value ≈ -0.001
        assert pa.types.is_float64(table.schema.field("SMALL_NEG").type)
        assert_float_equal(table.column("SMALL_NEG")[0].as_py(), -0.001)

        # And The NULL_NUM column should contain None
        assert table.column("NULL_NUM")[0].as_py() is None

    def test_should_convert_scaled_fixed_number_to_decimal(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When A query with a scaled NUMBER column is executed
        cursor.execute("SELECT 3.14::NUMBER(10,2) AS pi")

        # And Results are collected with number_to_decimal=True
        stream_ptr = cursor._get_stream_ptr()
        arrow_context = ArrowConverterContext()
        table = pa.Table.from_batches(list(ArrowStreamTableIterator(stream_ptr, arrow_context, number_to_decimal=True)))

        # Then The column should be Decimal128
        assert pa.types.is_decimal128(table.schema.field("PI").type)

    def test_should_convert_timestamp_ntz(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When A query with a TIMESTAMP_NTZ column is executed
        cursor.execute("SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ AS ts, NULL::TIMESTAMP_NTZ AS null_ts")

        # And Results are collected
        table = _collect_as_table(cursor)

        # Then The column should be an Arrow timestamp type
        assert pa.types.is_timestamp(table.schema.field("TS").type)

        # And The value should represent 2024-01-15 10:30:00
        ts_val = table.column("TS")[0].as_py()
        assert ts_val.year == 2024
        assert ts_val.month == 1
        assert ts_val.day == 15
        assert ts_val.hour == 10
        assert ts_val.minute == 30
        assert ts_val.second == 0

        # And The null should be preserved
        assert table.column("NULL_TS")[0].as_py() is None

    def test_should_convert_time(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When A query with a TIME column is executed
        cursor.execute("SELECT '12:34:56'::TIME AS t, NULL::TIME AS null_t")

        # And Results are collected
        table = _collect_as_table(cursor)

        # Then The column should be an Arrow time type
        assert pa.types.is_time(table.schema.field("T").type)

        # And The value should represent 12:34:56
        t_val = table.column("T")[0].as_py()
        assert t_val == time(12, 34, 56)

        # And The null should be preserved
        assert table.column("NULL_T")[0].as_py() is None

    def test_should_pass_through_types_not_needing_conversion(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When A query with types that need no conversion is executed
        cursor.execute(
            "SELECT 42::NUMBER(10,0) AS int_val, 'hello'::VARCHAR AS str_val, "
            "TRUE AS bool_val, '2024-06-15'::DATE AS date_val"
        )

        # And Results are collected
        table = _collect_as_table(cursor)

        # Then Integer column should pass through as-is
        assert table.column("INT_VAL")[0].as_py() == 42

        # And String column should pass through as-is
        assert table.column("STR_VAL")[0].as_py() == "hello"

        # And Boolean column should pass through as-is
        assert table.column("BOOL_VAL")[0].as_py() is True

        # And Date column should pass through as-is
        date_val = table.column("DATE_VAL")[0].as_py()
        assert date_val.year == 2024
        assert date_val.month == 6
        assert date_val.day == 15

    def test_should_handle_mixed_types_with_nulls(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A table with multiple types exists
        table_name = f"{tmp_schema}.test_dummy_mixed"
        cursor.execute(
            f"CREATE TABLE {table_name} (  id NUMBER(10,0),  score NUMBER(10,2),  name VARCHAR,  ts TIMESTAMP_NTZ)"
        )

        # And Rows with mixed values and NULLs are inserted
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 9.75, 'Alice', '2024-01-01 00:00:00')")
        cursor.execute(f"INSERT INTO {table_name} VALUES (2, NULL, NULL, NULL)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (3, -0.50, 'Bob', '2025-12-31 23:59:59')")

        # When The table is queried and collected via ArrowStreamTableIterator
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        table = _collect_as_table(cursor)

        # Then There should be 3 rows
        assert table.num_rows == 3

        # And The SCORE column should be converted to float64
        assert pa.types.is_float64(table.schema.field("SCORE").type)
        assert_float_equal(table.column("SCORE")[0].as_py(), 9.75)
        assert table.column("SCORE")[1].as_py() is None
        assert_float_equal(table.column("SCORE")[2].as_py(), -0.50)

        # And The TS column should be converted to Arrow timestamp
        assert pa.types.is_timestamp(table.schema.field("TS").type)
        assert table.column("TS")[1].as_py() is None
        ts_val = table.column("TS")[2].as_py()
        assert ts_val.year == 2025
        assert ts_val.month == 12
        assert ts_val.day == 31
