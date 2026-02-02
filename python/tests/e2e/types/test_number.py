"""NUMBER type tests for Universal Driver.

This module tests NUMBER type and its synonyms (DECIMAL, NUMERIC)
across various scenarios including literals, table operations, precision/scale boundaries,
NULL handling, parameter binding, large result sets, and type casting.

All tests are parameterized to run with each type synonym to verify they behave identically.

CRITICAL TYPE MAPPING RULE:
- scale = 0: Returns Python `int` (even for 38-digit values)
- scale > 0: Returns Python `Decimal` (regardless of scale value)

NUMBER provides exact 38-digit precision with fixed scale (0-37), contrasting with
FLOAT's binary approximation and DECFLOAT's dynamic scale.
"""

from __future__ import annotations

from decimal import Decimal

import pytest

from .utils import assert_sequential_values, assert_type


# =============================================================================
# TYPE SYNONYMS
# =============================================================================
# https://docs.snowflake.com/en/sql-reference/data-types-numeric
# Note: INT, INTEGER, BIGINT, etc. are NOT tested here (tested in test_int.py)
NUMBER_TYPE_SYNONYMS = [
    "NUMBER",
    "DECIMAL",
    "NUMERIC",
]
number_type_parametrize = pytest.mark.parametrize("num_type", NUMBER_TYPE_SYNONYMS)

# =============================================================================
# 38-DIGIT PRECISION VALUES
# =============================================================================
# Maximum 38-digit integer (scale=0 → int)
NUMBER_38_DIGITS_INT = 12345678901234567890123456789012345678
# 38 total digits with scale=2 (scale>0 → Decimal)
NUMBER_38_DIGITS_SCALE2 = Decimal("123456789012345678901234567890123456.78")
# 38 total digits with scale=10 (scale>0 → Decimal)
NUMBER_38_DIGITS_SCALE10 = Decimal("1234567890123456789012345678.1234567890")
# 38 total digits with maximum scale=37 (scale>0 → Decimal)
NUMBER_38_DIGITS_SCALE37 = Decimal("1.2345678901234567890123456789012345678")

# =============================================================================
# PRECISION/SCALE BOUNDARY VALUES
# =============================================================================
# NUMBER(5,2): 3 digits left, 2 right - boundary is 999.99
NUMBER_5_2_MAX = Decimal("999.99")
NUMBER_5_2_MIN = Decimal("-999.99")
# NUMBER(8,0): 8-digit integer - boundary is 99999999 (used in table tests)
NUMBER_8_0_MAX = 99999999
NUMBER_8_0_MIN = -99999999
# NUMBER(38,0): Max 38-digit integer (same as INT max precision)
NUMBER_38_0_MAX = 99999999999999999999999999999999999999
NUMBER_38_0_MIN = -99999999999999999999999999999999999999
# NUMBER(38,37): 1 digit left, 37 right - smallest positive non-zero
NUMBER_38_37_MIN_POSITIVE = Decimal("0.0000000000000000000000000000000000001")

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 30_000


class TestNumberTypeCasting:
    """Tests for NUMBER type casting to appropriate type based on scale."""

    @number_type_parametrize
    def test_should_cast_number_values_to_appropriate_type_for_number_and_synonyms(self, execute_query, num_type):
        # Given Snowflake client is logged in

        # When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2), 123.45::<type>(10,2)" is executed
        sql = f"SELECT 0::{num_type}(10,0), 123::{num_type}(10,0), 0.00::{num_type}(10,2), 123.45::{num_type}(10,2)"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result[:2], int)  # scale=0 → int
        assert_type(result[2:], Decimal)  # scale>0 → Decimal

        # And Values should match [0, 123, 0.00, 123.45]
        assert result == (0, 123, Decimal("0.00"), Decimal("123.45"))


class TestNumberLiteral:
    """Tests for NUMBER type using SELECT with literals (no tables)."""

    @number_type_parametrize
    def test_should_select_number_literals_for_number_and_synonyms(self, execute_query, num_type):
        # Given Snowflake client is logged in

        # When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2), -123.45::<type>(10,2),
        # 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
        sql = (
            f"SELECT 0::{num_type}(10,0), -456::{num_type}(10,0), "
            f"1.50::{num_type}(10,2), -123.45::{num_type}(10,2), "
            f"123.456::{num_type}(15,3), -789.012::{num_type}(15,3)"
        )
        result = execute_query(sql, single_row=True)

        # Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]
        expected = (0, -456, Decimal("1.50"), Decimal("-123.45"), Decimal("123.456"), Decimal("-789.012"))
        # Python: scale=0 -> int, scale>0 -> Decimal
        assert result == expected
        assert_type(result[:2], int)  # scale=0 → int
        assert_type(result[2:], Decimal)  # scale>0 → Decimal

    @number_type_parametrize
    def test_should_handle_high_precision_values_from_literals_for_number_and_synonyms(self, execute_query, num_type):
        # Given Snowflake client is logged in

        # When Query "SELECT 12345678901234567890123456789012345678::<type>(38,0),
        # 123456789012345678901234567890123456.78::<type>(38,2),
        # 1234567890123456789012345678.1234567890::<type>(38,10),
        # 0.0000000000000000000000000000000000001::<type>(38,37)" is executed
        sql = (
            f"SELECT {NUMBER_38_DIGITS_INT}::{num_type}(38,0), "
            f"{NUMBER_38_DIGITS_SCALE2}::{num_type}(38,2), "
            f"{NUMBER_38_DIGITS_SCALE10}::{num_type}(38,10), "
            f"{NUMBER_38_37_MIN_POSITIVE}::{num_type}(38,37)"
        )
        result = execute_query(sql, single_row=True)

        # Then Result should contain [12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890,
        # 0.0000000000000000000000000000000000001]
        expected = (NUMBER_38_DIGITS_INT, NUMBER_38_DIGITS_SCALE2, NUMBER_38_DIGITS_SCALE10, NUMBER_38_37_MIN_POSITIVE)
        # Python: scale=0 -> int, scale>0 -> Decimal
        assert result == expected
        assert_type([result[0]], int)
        assert_type(result[1:], Decimal)

    @number_type_parametrize
    def test_should_handle_scale_and_precision_boundaries_from_literals_for_number_and_synonyms(
        self, execute_query, num_type
    ):
        # Given Snowflake client is logged in

        # When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2), 99999999::<type>(8,0),
        # -99999999::<type>(8,0)" is executed
        sql = (
            f"SELECT {NUMBER_5_2_MAX}::{num_type}(5,2), {NUMBER_5_2_MIN}::{num_type}(5,2), "
            f"{NUMBER_8_0_MAX}::{num_type}(8,0), {NUMBER_8_0_MIN}::{num_type}(8,0)"
        )
        result = execute_query(sql, single_row=True)

        # Then Result should contain [999.99, -999.99, 99999999, -99999999]
        assert result == (NUMBER_5_2_MAX, NUMBER_5_2_MIN, NUMBER_8_0_MAX, NUMBER_8_0_MIN)
        # Python: scale>0 -> Decimal, scale=0 -> int
        assert_type(result[:2], Decimal)
        assert_type(result[2:], int)

    @number_type_parametrize
    def test_should_handle_high_precision_boundaries_from_literals_for_number_and_synonyms(
        self, execute_query, num_type
    ):
        # Given Snowflake client is logged in

        # When Query "SELECT 99999999999999999999999999999999999999::<type>(38,0),
        # -99999999999999999999999999999999999999::<type>(38,0)" is executed
        sql = f"SELECT {NUMBER_38_0_MAX}::{num_type}(38,0), {NUMBER_38_0_MIN}::{num_type}(38,0)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain max and min 38-digit integers
        assert result == (NUMBER_38_0_MAX, NUMBER_38_0_MIN)
        # Python: scale=0 -> int
        assert_type(result, int)

    @number_type_parametrize
    def test_should_handle_null_values_from_literals_for_number_and_synonyms(self, execute_query, num_type):
        # Given Snowflake client is logged in

        # When Query "SELECT NULL::<type>(10,0), 42::<type>(10,0), NULL::<type>(10,2), 42.50::<type>(10,2)" is executed
        sql = f"SELECT NULL::{num_type}(10,0), 42::{num_type}(10,0), NULL::{num_type}(10,2), 42.50::{num_type}(10,2)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [NULL, 42, NULL, 42.50]
        assert result == (None, 42, None, Decimal("42.50"))
        # Python: scale=0 -> int, scale>0 -> Decimal
        assert_type(result[:2], int, can_be_none=True)
        assert_type(result[2:], Decimal, can_be_none=True)

    @number_type_parametrize
    def test_should_download_large_result_set_with_multiple_chunks_from_generator_for_number_and_synonyms(
        self, execute_query, num_type
    ):
        # Given Snowflake client is logged in

        # When Query
        # "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5) FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v"
        # is executed

        # Note: seq8() doesn't guarantee consecutive values in parallel execution,
        # so we use ROW_NUMBER() to ensure sequential integers.
        sql = (
            f"WITH base AS ("
            f"  SELECT ROW_NUMBER() OVER (ORDER BY seq8()) - 1 as rn "
            f"  FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
            f") "
            f"SELECT rn::{num_type}(38,0), (rn + 0.12345)::{num_type}(20,5) FROM base "
            f"ORDER BY 1"
        )
        rows = execute_query(sql)

        # Then Column 1 should contain sequential integers from 0 to 29999
        col0_values = [row[0] for row in rows]
        # Python: scale=0 -> int
        assert_type(col0_values, int)
        assert_sequential_values(col0_values, LARGE_RESULT_SET_SIZE)

        # And Column 2 should contain sequential decimals starting from 0.12345
        col1_values = [row[1] for row in rows]
        # Python: scale>0 -> Decimal
        assert_type(col1_values, Decimal)
        assert_sequential_values(
            col1_values,
            LARGE_RESULT_SET_SIZE,
            transform=lambda i: Decimal(str(i)) + Decimal("0.12345"),
        )


class TestNumberTable:
    """Tests for NUMBER type using table operations."""

    @number_type_parametrize
    def test_should_select_numbers_from_table_with_multiple_scales_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
        table_name = f"{tmp_schema}.number_table_{num_type.lower()}"
        execute_query(
            f"CREATE TABLE {table_name} ("
            f"col_scale0 {num_type}(10,0), "
            f"col_scale2 {num_type}(10,2), "
            f"col_scale3 {num_type}(15,3), "
            f"col_scale5 {num_type}(20,5))"
        )

        # And Row (123, 123.45, 123.456, 12345.67890) is inserted
        # And Row (-456, -67.89, -789.012, -98765.43210) is inserted
        # And Row (0, 0.00, 0.000, 0.00000) is inserted
        # And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
        test_data = [
            (123, Decimal("123.45"), Decimal("123.456"), Decimal("12345.67890")),
            (-456, Decimal("-67.89"), Decimal("-789.012"), Decimal("-98765.43210")),
            (0, Decimal("0.00"), Decimal("0.000"), Decimal("0.00000")),
            (999999, Decimal("999.99"), Decimal("1000.500"), Decimal("123456.78901")),
        ]
        for row in test_data:
            execute_query(f"INSERT INTO {table_name} VALUES ({row[0]}, {row[1]}, {row[2]}, {row[3]})")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain 4 rows with expected values
        assert len(rows) == 4
        assert rows == test_data
        for row in rows:
            assert_type(row[:1], int)
            assert_type(row[1:], Decimal)

    @number_type_parametrize
    def test_should_handle_high_precision_values_from_table_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(38,0), <type>(38,2), <type>(38,10), <type>(38,37)) exists
        table_name = f"{tmp_schema}.precision_table_{num_type.lower()}"
        execute_query(
            f"CREATE TABLE {table_name} ("
            f"col_38_0 {num_type}(38,0), "
            f"col_38_2 {num_type}(38,2), "
            f"col_38_10 {num_type}(38,10), "
            f"col_38_37 {num_type}(38,37))"
        )

        # And Row (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78,
        # 1234567890123456789012345678.1234567890, 1.2345678901234567890123456789012345678) is inserted
        execute_query(
            f"INSERT INTO {table_name} VALUES ("
            f"{NUMBER_38_DIGITS_INT}, "
            f"{NUMBER_38_DIGITS_SCALE2}, "
            f"{NUMBER_38_DIGITS_SCALE10}, "
            f"{NUMBER_38_DIGITS_SCALE37})"
        )

        # When Query "SELECT * FROM <table>" is executed
        result = execute_query(f"SELECT * FROM {table_name}", single_row=True)

        # Then Result should contain [12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78, 1234567890123456789012345678.1234567890,
        # 1.2345678901234567890123456789012345678]
        assert result == (
            NUMBER_38_DIGITS_INT,
            NUMBER_38_DIGITS_SCALE2,
            NUMBER_38_DIGITS_SCALE10,
            NUMBER_38_DIGITS_SCALE37,
        )
        # Python: scale=0 -> int, scale>0 -> Decimal
        assert_type(result[:1], int)
        assert_type(result[1:], Decimal)

    @number_type_parametrize
    def test_should_handle_scale_and_precision_boundaries_from_table_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(5,2), <type>(8,0)) exists
        table_name = f"{tmp_schema}.boundary_table_{num_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col_5_2 {num_type}(5,2), col_8_0 {num_type}(8,0))")

        # And Row (999.99, 99999999) is inserted
        # And Row (-999.99, -99999999) is inserted
        # And Row (123.45, 12345678) is inserted
        # And Row (0.01, 0) is inserted
        test_data = [
            (NUMBER_5_2_MAX, NUMBER_8_0_MAX),
            (NUMBER_5_2_MIN, NUMBER_8_0_MIN),
            (Decimal("123.45"), 12345678),
            (Decimal("0.01"), 0),
        ]
        for row in test_data:
            execute_query(f"INSERT INTO {table_name} VALUES ({row[0]}, {row[1]})")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain 4 rows with expected boundary values
        assert len(rows) == 4
        assert rows == test_data
        for row in rows:
            assert_type(row[:1], Decimal)
            assert_type(row[1:], int)

    @number_type_parametrize
    def test_should_handle_high_precision_boundaries_from_table_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(38,0), <type>(38,37)) exists
        table_name = f"{tmp_schema}.high_precision_boundary_table_{num_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col_38_0 {num_type}(38,0), col_38_37 {num_type}(38,37))")

        # And Row (99999999999999999999999999999999999999, 1.2345678901234567890123456789012345678) is inserted
        # And Row (-99999999999999999999999999999999999999, -1.2345678901234567890123456789012345678) is inserted
        # And Row (12345678901234567890123456789012345678, 0.0000000000000000000000000000000000001) is inserted
        test_data = [
            (NUMBER_38_0_MAX, NUMBER_38_DIGITS_SCALE37),
            (NUMBER_38_0_MIN, -NUMBER_38_DIGITS_SCALE37),
            (NUMBER_38_DIGITS_INT, NUMBER_38_37_MIN_POSITIVE),
        ]
        for row in test_data:
            execute_query(f"INSERT INTO {table_name} VALUES ({row[0]}, {row[1]})")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain 3 rows with expected high precision boundary values
        assert len(rows) == 3
        assert rows == test_data
        for row in rows:
            assert_type(row[:1], int)
            assert_type(row[1:], Decimal)

    @number_type_parametrize
    def test_should_handle_null_values_from_table_with_multiple_scales_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3)) exists
        table_name = f"{tmp_schema}.null_table_{num_type.lower()}"
        execute_query(
            f"CREATE TABLE {table_name} ("
            f"col_10_0 {num_type}(10,0), "
            f"col_10_2 {num_type}(10,2), "
            f"col_15_3 {num_type}(15,3))"
        )

        # And Row (NULL, NULL, NULL) is inserted
        # And Row (123, 123.45, 123.456) is inserted
        # And Row (NULL, NULL, NULL) is inserted
        # And Row (-456, -67.89, -789.012) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (NULL, NULL, NULL)")
        execute_query(f"INSERT INTO {table_name} VALUES (123, 123.45, 123.456)")
        execute_query(f"INSERT INTO {table_name} VALUES (NULL, NULL, NULL)")
        execute_query(f"INSERT INTO {table_name} VALUES (-456, -67.89, -789.012)")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain 4 rows with 2 NULL rows and 2 non-NULL rows with expected values
        expected = [
            (None, None, None),
            (123, Decimal("123.45"), Decimal("123.456")),
            (None, None, None),
            (-456, Decimal("-67.89"), Decimal("-789.012")),
        ]
        assert len(rows) == 4
        assert rows == expected
        for row in rows:
            assert_type(row[:1], int, can_be_none=True)
            assert_type(row[1:2], Decimal, can_be_none=True)
            assert_type(row[2:], Decimal, can_be_none=True)

    @number_type_parametrize
    def test_should_download_large_result_set_from_table_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows,
        # from 0 to 29999 in the first column and from 0.12345 to 29999.12345 in the second column

        # Note: seq8() doesn't guarantee consecutive values in parallel execution,
        # so we use ROW_NUMBER() to ensure sequential integers.
        table_name = f"{tmp_schema}.large_table_{num_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col1 {num_type}(38,0), col2 {num_type}(20,5))")
        execute_query(
            f"INSERT INTO {table_name} "
            f"WITH base AS ("
            f"  SELECT ROW_NUMBER() OVER (ORDER BY seq8()) - 1 as rn "
            f"  FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
            f") "
            f"SELECT rn::{num_type}(38,0), (rn + 0.12345)::{num_type}(20,5) FROM base"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY 1")

        # Then Column 1 should contain sequential integers from 0 to 29999
        col1 = [row[0] for row in rows]
        # Python: scale=0 -> int
        assert_type(col1, int)
        assert_sequential_values(col1, LARGE_RESULT_SET_SIZE)

        # And Column 2 should contain sequential decimals starting from 0.12345
        col2 = [row[1] for row in rows]
        # Python: scale>0 -> Decimal
        assert_type(col2, Decimal)
        assert_sequential_values(
            col2,
            LARGE_RESULT_SET_SIZE,
            transform=lambda i: Decimal(f"{i}.12345"),
        )


class TestNumberBinding:
    """Tests for NUMBER type using parameter binding."""

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @number_type_parametrize
    def test_should_select_number_using_parameter_binding_for_number_and_synonyms(self, execute_query, num_type):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::<type>(10,0), ?::<type>(10,0), ?::<type>(10,2), ?::<type>(10,2), ?::<type>(10,0)"
        # is executed with bound values [123, -456, 12.34, -56.78, NULL]
        sql = (
            f"SELECT ?::{num_type}(10,0), ?::{num_type}(10,0), "
            f"?::{num_type}(10,2), ?::{num_type}(10,2), ?::{num_type}(10,0)"
        )
        result = execute_query(
            sql,
            (123, -456, Decimal("12.34"), Decimal("-56.78"), None),
            single_row=True,
        )

        # Then Result should contain [123, -456, 12.34, -56.78, NULL]
        assert result == (123, -456, Decimal("12.34"), Decimal("-56.78"), None)
        assert_type(result[:2], int)
        assert_type(result[2:], Decimal, can_be_none=True)

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @number_type_parametrize
    def test_should_select_high_precision_number_using_parameter_binding_for_number_and_synonyms(
        self, execute_query, num_type
    ):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::<type>(38,0), ?::<type>(38,2)" is executed
        # with bound values [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78]
        sql = f"SELECT ?::{num_type}(38,0), ?::{num_type}(38,2)"
        result = execute_query(
            sql,
            (NUMBER_38_DIGITS_INT, NUMBER_38_DIGITS_SCALE2),
            single_row=True,
        )

        # Then Result should contain [12345678901234567890123456789012345678, 123456789012345678901234567890123456.78]
        assert result == (NUMBER_38_DIGITS_INT, NUMBER_38_DIGITS_SCALE2)
        # Python: scale=0 -> int, scale>0 -> Decimal
        assert_type([result[0]], int)
        assert_type([result[1]], Decimal)

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @number_type_parametrize
    def test_should_insert_number_using_parameter_binding_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(10,0), <type>(10,2)) exists
        table_name = f"{tmp_schema}.number_bind_{num_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col_int {num_type}(10,0), col_dec {num_type}(10,2))")

        # When Rows (0, 0.00), (123, 123.45), (-456, -67.89), (999999, 999.99), (NULL, NULL) are inserted using binding
        test_data = [
            (0, Decimal("0.00")),
            (123, Decimal("123.45")),
            (-456, Decimal("-67.89")),
            (999999, Decimal("999.99")),
            (None, None),
        ]
        for row in test_data:
            execute_query(f"INSERT INTO {table_name} VALUES (?, ?)", row)

        # Then Result should contain 5 rows with expected values
        rows = execute_query(f"SELECT * FROM {table_name}")
        assert len(rows) == 5
        col_int_values = [row[0] for row in rows]
        col_dec_values = [row[1] for row in rows]
        # Python: scale=0 -> int, scale>0 -> Decimal
        assert_type(col_int_values, int, can_be_none=True)
        assert_type(col_dec_values, Decimal, can_be_none=True)
        assert set(col_int_values) == {0, 123, -456, 999999, None}
        assert set(col_dec_values) == {Decimal("0.00"), Decimal("123.45"), Decimal("-67.89"), Decimal("999.99"), None}

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @number_type_parametrize
    def test_should_insert_high_precision_number_using_parameter_binding_for_number_and_synonyms(
        self, execute_query, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in

        # And Table with columns (<type>(38,0), <type>(38,2)) exists
        table_name = f"{tmp_schema}.high_precision_bind_{num_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col_38_0 {num_type}(38,0), col_38_2 {num_type}(38,2))")

        # When Rows (12345678901234567890123456789012345678, 123456789012345678901234567890123456.78),
        # (99999999999999999999999999999999999999, 0.01), (-99999999999999999999999999999999999999, -0.01)
        # are inserted using binding
        test_data = [
            (NUMBER_38_DIGITS_INT, NUMBER_38_DIGITS_SCALE2),
            (NUMBER_38_0_MAX, Decimal("0.01")),
            (NUMBER_38_0_MIN, Decimal("-0.01")),
        ]
        for row in test_data:
            execute_query(f"INSERT INTO {table_name} VALUES (?, ?)", row)

        # Then Result should contain 3 rows with expected values keeping the precision
        rows = execute_query(f"SELECT * FROM {table_name}")
        assert len(rows) == 3
        assert rows == test_data
        for row in rows:
            # Python: scale=0 -> int, scale>0 -> Decimal
            assert_type([row[0]], int)
            assert_type([row[1]], Decimal)
