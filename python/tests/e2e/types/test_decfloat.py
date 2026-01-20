"""DECFLOAT type tests for Universal Driver.

This module tests DECFLOAT type which provides exact 38-digit decimal precision
with a wide exponent range (-16383 to +16384).

DECFLOAT has NO type synonyms (unlike FLOAT).
DECFLOAT does NOT support special values (NaN, inf, -inf) unlike FLOAT.

All values are returned as Python Decimal type with full 38-digit precision.
"""

from __future__ import annotations

from decimal import Decimal

import pytest

from .utils import assert_type


# =============================================================================
# PRECISION TEST VALUES (38 significant decimal digits)
# =============================================================================
# Plain 38-digit integer
DECFLOAT_38_DIGITS = Decimal("12345678901234567890123456789012345678")
# 38 digits with positive exponent (very large number)
DECFLOAT_38_DIGITS_POS_EXP = Decimal("1.2345678901234567890123456789012345678E+100")
# 38 digits with negative exponent (very small number)
DECFLOAT_38_DIGITS_NEG_EXP = Decimal("1.2345678901234567890123456789012345678E-100")

# =============================================================================
# EXTREME EXPONENT VALUES
# =============================================================================
# DECFLOAT exponent range: -16383 to +16384 (vastly exceeds FLOAT's ~±308)
DECFLOAT_MAX_EXPONENT = Decimal("1E+16384")
DECFLOAT_MIN_EXPONENT = Decimal("1E-16383")
# Large positive/negative exponents for testing
DECFLOAT_LARGE_POS_EXPONENT = Decimal("-1.234E+8000")
DECFLOAT_LARGE_NEG_EXPONENT = Decimal("9.876E-8000")

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 1_000_000


class TestDecfloatTypeCasting:
    """Tests for DECFLOAT type casting to appropriate type."""

    def test_should_cast_decfloat_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT,
        # '12345678901234567890123456789012345678'::DECFLOAT" is executed
        sql = f"SELECT 0::DECFLOAT, 123.456::DECFLOAT, 1.23e37::DECFLOAT, '{DECFLOAT_38_DIGITS}'::DECFLOAT"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, Decimal)

        # And Values should maintain full 38-digit precision
        assert result == (Decimal("0"), Decimal("123.456"), Decimal("1.23E+37"), DECFLOAT_38_DIGITS)


class TestDecfloatLiteral:
    """Tests for DECFLOAT type using SELECT with literals (no tables)."""

    def test_should_select_decfloat_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT,
        # 123.456789::DECFLOAT, -987.654321::DECFLOAT" is executed
        sql = "SELECT 0::DECFLOAT, 1.5::DECFLOAT, -1.5::DECFLOAT, 123.456789::DECFLOAT, -987.654321::DECFLOAT"
        result = execute_query(sql, single_row=True)

        # Then Result should contain exact decimals [0, 1.5, -1.5, 123.456789, -987.654321]
        expected = (
            Decimal("0"),
            Decimal("1.5"),
            Decimal("-1.5"),
            Decimal("123.456789"),
            Decimal("-987.654321"),
        )
        assert result == expected
        assert_type(result, Decimal)

    def test_should_handle_full_38_digit_precision_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT '12345678901234567890123456789012345678'::DECFLOAT,
        # '1.2345678901234567890123456789012345678E+100'::DECFLOAT,
        # '1.2345678901234567890123456789012345678E-100'::DECFLOAT" is executed
        sql = (
            f"SELECT '{DECFLOAT_38_DIGITS}'::DECFLOAT, "
            f"'{DECFLOAT_38_DIGITS_POS_EXP}'::DECFLOAT, "
            f"'{DECFLOAT_38_DIGITS_NEG_EXP}'::DECFLOAT"
        )
        result = execute_query(sql, single_row=True)

        # Then Result should preserve all 38 digits for each value
        assert result == (DECFLOAT_38_DIGITS, DECFLOAT_38_DIGITS_POS_EXP, DECFLOAT_38_DIGITS_NEG_EXP)
        assert_type(result, Decimal)

    def test_should_handle_extreme_exponent_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT '1E+16384'::DECFLOAT, '1E-16383'::DECFLOAT" is executed
        result = execute_query(
            f"SELECT '{DECFLOAT_MAX_EXPONENT}'::DECFLOAT, '{DECFLOAT_MIN_EXPONENT}'::DECFLOAT",
            single_row=True,
        )
        # Then Result should contain [1E+16384, 1E-16383]
        assert result == (DECFLOAT_MAX_EXPONENT, DECFLOAT_MIN_EXPONENT)
        assert_type(result, Decimal)

        # When Query "SELECT '-1.234E+8000'::DECFLOAT, '9.876E-8000'::DECFLOAT" is executed
        result = execute_query(
            f"SELECT '{DECFLOAT_LARGE_POS_EXPONENT}'::DECFLOAT, '{DECFLOAT_LARGE_NEG_EXPONENT}'::DECFLOAT",
            single_row=True,
        )
        # Then Result should contain [-1.234E+8000, 9.876E-8000]
        assert result == (DECFLOAT_LARGE_POS_EXPONENT, DECFLOAT_LARGE_NEG_EXPONENT)
        assert_type(result, Decimal)

    def test_should_handle_null_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT" is executed
        result = execute_query("SELECT NULL::DECFLOAT, 42.5::DECFLOAT, NULL::DECFLOAT", single_row=True)

        # Then Result should contain [NULL, 42.5, NULL]
        assert result == (None, Decimal("42.5"), None)
        assert_type(result, Decimal, can_be_none=True)

    def test_should_download_large_result_set_with_multiple_chunks_from_generator(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v" is executed
        sql = f"SELECT seq8()::DECFLOAT as id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        rows = execute_query(sql)

        # Then Result should contain consecutive numbers from 0 to 999999
        values = [row[0] for row in rows]
        assert values == [Decimal(i) for i in range(LARGE_RESULT_SET_SIZE)]

        # And All values should be returned as appropriate type
        assert_type(values, Decimal)


class TestDecfloatTable:
    """Tests for DECFLOAT type using table operations."""

    def test_should_select_decfloats_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with DECFLOAT column exists with values [0, 123.456, -789.012, 1.23e20, -9.87e-15]
        table_name = f"{tmp_schema}.decfloat_table"
        execute_query(f"CREATE TABLE {table_name} (col DECFLOAT)")
        test_values = [
            Decimal("0"),
            Decimal("123.456"),
            Decimal("-789.012"),
            Decimal("1.23E+20"),
            Decimal("-9.87E-15"),
        ]
        for val in test_values:
            execute_query(f"INSERT INTO {table_name} VALUES ('{val}')")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain exact decimals [0, 123.456, -789.012, 1.23e20, -9.87e-15]
        result = [row[0] for row in rows]
        assert result == test_values
        assert_type(result, Decimal)

    def test_should_handle_full_38_digit_precision_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with DECFLOAT column exists with values
        # [12345678901234567890123456789012345678,
        # 1.2345678901234567890123456789012345678E+100,
        # 1.2345678901234567890123456789012345678E-100]
        table_name = f"{tmp_schema}.precision_table"
        execute_query(f"CREATE TABLE {table_name} (col DECFLOAT)")
        precision_values = [DECFLOAT_38_DIGITS, DECFLOAT_38_DIGITS_POS_EXP, DECFLOAT_38_DIGITS_NEG_EXP]
        for val in precision_values:
            execute_query(f"INSERT INTO {table_name} VALUES ('{val}')")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should preserve all 38 digits for each value
        result = [row[0] for row in rows]
        assert result == precision_values
        assert_type(result, Decimal)

    def test_should_handle_extreme_exponent_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with DECFLOAT column exists with values
        # [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
        table_name = f"{tmp_schema}.extreme_table"
        execute_query(f"CREATE TABLE {table_name} (col DECFLOAT)")
        extreme_values = [
            DECFLOAT_MAX_EXPONENT,
            DECFLOAT_MIN_EXPONENT,
            DECFLOAT_LARGE_POS_EXPONENT,
            DECFLOAT_LARGE_NEG_EXPONENT,
        ]
        for val in extreme_values:
            execute_query(f"INSERT INTO {table_name} VALUES ('{val}')")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain [1E+16384, 1E-16383, -1.234E+8000, 9.876E-8000]
        result = [row[0] for row in rows]
        assert result == extreme_values
        assert_type(result, Decimal)

    def test_should_handle_null_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with DECFLOAT column exists with values [NULL, 123.456, NULL, -789.012]
        table_name = f"{tmp_schema}.null_table"
        execute_query(f"CREATE TABLE {table_name} (col DECFLOAT)")
        execute_query(f"INSERT INTO {table_name} VALUES (NULL), (123.456), (NULL), (-789.012)")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")
        values = [row[0] for row in rows]

        # Then Result should contain [NULL, 123.456, NULL, -789.012]
        assert values == [None, Decimal("123.456"), None, Decimal("-789.012")]
        assert_type(values, Decimal, can_be_none=True)

    def test_should_download_large_result_set_with_multiple_chunks_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with DECFLOAT column exists with values from 0 to 999999
        table_name = f"{tmp_schema}.large_table"
        execute_query(f"CREATE TABLE {table_name} (col DECFLOAT)")
        execute_query(
            f"INSERT INTO {table_name} SELECT seq8()::DECFLOAT "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain consecutive numbers from 0 to 999999
        values = [row[0] for row in rows]
        assert values == [Decimal(i) for i in range(LARGE_RESULT_SET_SIZE)]

        # And All values should be returned as appropriate type
        assert_type(values, Decimal)


class TestDecfloatBinding:
    """Tests for DECFLOAT type using parameter binding."""

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    def test_should_select_decfloat_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT" is executed
        # with bound DECFLOAT values [123.456, -789.012, 42.0]
        result = execute_query(
            "SELECT ?::DECFLOAT, ?::DECFLOAT, ?::DECFLOAT",
            (("DECFLOAT", Decimal("123.456")), ("DECFLOAT", Decimal("-789.012")), ("DECFLOAT", Decimal("42.0"))),
            single_row=True,
        )

        # Then Result should contain [123.456, -789.012, 42.0]
        assert result == (Decimal("123.456"), Decimal("-789.012"), Decimal("42.0"))
        assert_type(result, Decimal)

        # When Query "SELECT ?::DECFLOAT" is executed with bound NULL value
        result = execute_query("SELECT ?::DECFLOAT", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    def test_should_select_extreme_decfloat_values_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::DECFLOAT" is executed with bound value 1E+16384
        result = execute_query(
            "SELECT ?::DECFLOAT",
            (("DECFLOAT", DECFLOAT_MAX_EXPONENT),),
            single_row=True,
        )

        # Then Result should contain [1E+16384]
        assert result == (DECFLOAT_MAX_EXPONENT,)
        assert_type(result, Decimal)

        # When Query "SELECT ?::DECFLOAT" is executed with bound value -1.234E+8000
        result = execute_query(
            "SELECT ?::DECFLOAT",
            (("DECFLOAT", DECFLOAT_LARGE_POS_EXPONENT),),
            single_row=True,
        )

        # Then Result should contain [-1.234E+8000]
        assert result == (DECFLOAT_LARGE_POS_EXPONENT,)
        assert_type(result, Decimal)

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    def test_should_insert_decfloat_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with DECFLOAT column exists
        table_name = f"{tmp_schema}.decfloat_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col DECFLOAT)")

        # When DECFLOAT values [0, 123.456, -789.012, NULL] are inserted using explicit binding
        test_values = [
            Decimal("0"),
            Decimal("123.456"),
            Decimal("-789.012"),
            None,
        ]
        for val in test_values:
            if val is None:
                execute_query(f"INSERT INTO {table_name} VALUES (?)", (None,))
            else:
                execute_query(f"INSERT INTO {table_name} VALUES (?)", (("DECFLOAT", val),))

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then SELECT should return the same exact values
        result = [row[0] for row in rows]
        assert result == test_values
        assert_type(result, Decimal, can_be_none=True)

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    def test_should_insert_extreme_decfloat_values_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with DECFLOAT column exists
        table_name = f"{tmp_schema}.decfloat_extreme_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col DECFLOAT)")

        # When DECFLOAT values [1E+16384, 1E-16383, -1.234E+8000] are inserted using explicit binding
        extreme_values = [
            DECFLOAT_MAX_EXPONENT,
            DECFLOAT_MIN_EXPONENT,
            DECFLOAT_LARGE_POS_EXPONENT,
        ]
        for val in extreme_values:
            execute_query(f"INSERT INTO {table_name} VALUES (?)", (("DECFLOAT", val),))

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then SELECT should return the same exact values
        result = [row[0] for row in rows]
        assert result == extreme_values
        assert_type(result, Decimal)
