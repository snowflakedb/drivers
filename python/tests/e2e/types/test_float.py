"""FLOAT type tests for Universal Driver.

This module tests FLOAT type and its synonyms (FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL)
across various scenarios including literals, table operations, special values (NaN, infinity),
boundary values, NULL handling, parameter binding, large result sets, and type casting.

All tests are parameterized to run with each type synonym to verify they behave identically.
All type synonyms are treated as 64-bit IEEE 754 double precision.
"""

from math import inf, nan

import pytest

from ...conftest import with_paramstyle
from .utils import (
    FLOAT_MIN_NORMAL,
    assert_floats_equal,
    assert_sequential_values,
    assert_type,
    floats_equal,
)


# =============================================================================
# TYPE SYNONYMS
# =============================================================================
# https://docs.snowflake.com/en/sql-reference/data-types-numeric
FLOAT_TYPE_SYNONYMS = [
    "FLOAT",
    "FLOAT4",
    "FLOAT8",
    "DOUBLE",
    "DOUBLE PRECISION",
    "REAL",
]
float_type_parametrize = pytest.mark.parametrize("float_type", FLOAT_TYPE_SYNONYMS)

# =============================================================================
# IEEE 754 BOUNDARY VALUES
# =============================================================================
# Maximum normalized positive value (approximately 1.8e308)
FLOAT_MAX = 1.7976931348623157e308
# Maximum normalized negative value
FLOAT_MIN = -1.7976931348623157e308
# Minimum subnormal positive value (smallest representable positive number)
FLOAT_MIN_SUBNORMAL = 5e-324

# =============================================================================
# PRECISION TEST VALUES
# =============================================================================
# IEEE 754 double precision has ~15.95 decimal digits of precision
# 15-digit value: exact representation guaranteed
FLOAT_15_DIGITS = 123456789012345.0
# 16-digit value: may have precision loss
FLOAT_16_DIGITS = 1234567890123456.0

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 50_000


class TestFloatTypeCasting:
    """Tests for FLOAT type casting to appropriate type."""

    @float_type_parametrize
    def test_should_cast_float_values_to_appropriate_type_for_float_and_synonyms(self, execute_query, float_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 0.0::<type>, 123.456::<type>, 1.23e10::<type>, 'NaN'::<type>, 'inf'::<type>" is executed
        sql = (
            f"SELECT 0.0::{float_type}, 123.456::{float_type}, 1.23e10::{float_type}, "
            f"'NaN'::{float_type}, 'inf'::{float_type}"
        )
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, float)

        # And Regular values should have approximately 15 decimal digits precision
        assert_floats_equal(result[:3], (0.0, 123.456, 1.23e10))

        # And NaN and inf values should be identified correctly
        assert_floats_equal(result[3:], (nan, inf))


class TestFloatLiteral:
    """Tests for FLOAT type using SELECT with literals (no tables)."""

    @float_type_parametrize
    def test_should_select_float_literals_for_float_and_synonyms(self, execute_query, float_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>, 123.456::<type>, -123.456::<type>" is executed
        sql = (
            f"SELECT 0.0::{float_type}, 1.0::{float_type}, -1.0::{float_type}, "
            f"123.456::{float_type}, -123.456::{float_type}"
        )
        result = execute_query(sql, single_row=True)

        # Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]
        assert_floats_equal(result, (0.0, 1.0, -1.0, 123.456, -123.456))
        assert_type(result, float)

    @float_type_parametrize
    def test_should_handle_special_float_values_from_literals_for_float_and_synonyms(self, execute_query, float_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
        sql = f"SELECT 'NaN'::{float_type}, 'inf'::{float_type}, '-inf'::{float_type}"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [NaN, positive_infinity, negative_infinity]
        assert_floats_equal(result, (nan, inf, -inf))
        assert_type(result, float)

    BOUNDARY_LITERAL_CASES = [
        ((FLOAT_MAX, FLOAT_MIN), (FLOAT_MAX, FLOAT_MIN)),
        ((FLOAT_MIN_NORMAL, FLOAT_MIN_SUBNORMAL), (FLOAT_MIN_NORMAL, FLOAT_MIN_SUBNORMAL)),
    ]

    @float_type_parametrize
    @pytest.mark.parametrize(
        "select_values, expected",
        BOUNDARY_LITERAL_CASES,
        ids=["max", "min"],
    )
    def test_should_handle_float_case_boundary_values_from_literals_for_float_and_synonyms(
        self, execute_query, float_type, select_values, expected
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        columns = ", ".join(f"{v}::{float_type}" for v in select_values)
        result = execute_query(f"SELECT {columns}", single_row=True)

        # Then Result should contain floats [<expected_values>]
        assert_floats_equal(result, expected)

    @float_type_parametrize
    def test_should_handle_float_precision_boundary_values_from_literals_for_float_and_synonyms(
        self, execute_query, float_type
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
        sql = f"SELECT {FLOAT_15_DIGITS}::{float_type}, {FLOAT_16_DIGITS}::{float_type}"
        result = execute_query(sql, single_row=True)

        # Then Result should verify precision around 15 decimal digits
        assert_floats_equal(result, (FLOAT_15_DIGITS, FLOAT_16_DIGITS))

    @float_type_parametrize
    def test_should_handle_null_values_from_literals_for_float_and_synonyms(self, execute_query, float_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
        sql = f"SELECT NULL::{float_type}, 42.5::{float_type}, NULL::{float_type}"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [NULL, 42.5, NULL]
        assert_floats_equal(result, (None, 42.5, None))
        assert_type(result, float, can_be_none=True)

    @float_type_parametrize
    def test_should_download_large_result_set_with_multiple_chunks_from_generator_for_float_and_synonyms(
        self, execute_query, float_type
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v" is executed
        sql = (
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::{float_type} as id "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY 1"
        )

        # Note: seq8() doesn't guarantee consecutive values in parallel execution,
        # so we use ROW_NUMBER() to ensure sequential integers.
        rows = execute_query(sql)

        # Then Result should contain 50000 rows with all values returned as appropriate float type
        values = [row[0] for row in rows]
        assert_type(values, float)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=float, compare=floats_equal)


class TestFloatTable:
    """Tests for FLOAT type using table operations."""

    @float_type_parametrize
    def test_should_select_floats_from_table_for_float_and_synonyms(self, execute_query, tmp_schema, float_type):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
        table_name = f"{tmp_schema}.float_table_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        test_values = [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
        for val in test_values:
            execute_query(f"INSERT INTO {table_name} VALUES ({val})")

        # When Query "SELECT * FROM float_table" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]
        result = [row[0] for row in rows]
        assert_floats_equal(result, [0.0, 123.456, -789.012, 123000.0, -0.00987])
        assert_type(result, float)

    @float_type_parametrize
    def test_should_handle_special_float_values_from_table_for_float_and_synonyms(
        self, execute_query, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with values [NaN, inf, -inf, 42.0, -42.0]
        table_name = f"{tmp_schema}.special_float_table_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        execute_query(
            f"INSERT INTO {table_name} VALUES\n"
            f"('NaN'::{float_type}),\n"
            f"('inf'::{float_type}),\n"
            f"('-inf'::{float_type}),\n"
            f"(42.0::{float_type}),\n"
            f"(-42.0::{float_type})"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")
        values = [row[0] for row in rows]

        # Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0]
        assert_floats_equal(values, [nan, inf, -inf, 42.0, -42.0])
        assert_type(values, float)

    @float_type_parametrize
    def test_should_handle_float_boundary_values_from_table_for_float_and_synonyms(
        self, execute_query, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with boundary values
        # [1.7976931348623157e308, -1.7976931348623157e308, 2.2250738585072014e-308, 5e-324, 123456789012345.0]
        table_name = f"{tmp_schema}.boundary_table_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        boundary_values = [
            FLOAT_MAX,
            FLOAT_MIN,
            FLOAT_MIN_NORMAL,
            FLOAT_MIN_SUBNORMAL,
            FLOAT_15_DIGITS,
        ]
        for val in boundary_values:
            execute_query(f"INSERT INTO {table_name} VALUES ({val})")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")
        result = [row[0] for row in rows]

        # Then Result should contain maximum, minimum, and precision boundary values
        # preserved within float precision limits
        assert_floats_equal(result, boundary_values)

    @float_type_parametrize
    def test_should_handle_null_values_from_table_for_float_and_synonyms(self, execute_query, tmp_schema, float_type):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
        table_name = f"{tmp_schema}.null_table_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        execute_query(f"INSERT INTO {table_name} VALUES (NULL), (123.456), (NULL), (-789.012)")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")
        values = [row[0] for row in rows]

        # Then Result should contain [NULL, 123.456, NULL, -789.012]
        assert_floats_equal(values, [None, 123.456, None, -789.012])
        assert_type(values, float, can_be_none=True)

    @float_type_parametrize
    def test_should_select_large_result_set_from_table_for_float_and_synonyms(
        self, execute_query, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with 50000 sequential values
        table_name = f"{tmp_schema}.large_float_table_{float_type.replace(' ', '_').lower()}"

        # Note: seq8() doesn't guarantee consecutive values in parallel execution,
        # so we use ROW_NUMBER() to ensure sequential integers.
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::{float_type} "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 50000 rows with all values returned as appropriate float type
        values = [row[0] for row in rows]
        assert_type(values, float)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=float, compare=floats_equal)


@with_paramstyle("qmark")
class TestFloatBinding:
    """Tests for FLOAT type using parameter binding."""

    @float_type_parametrize
    def test_should_select_float_using_parameter_binding_for_float_and_synonyms(self, execute_query, float_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::<type>, ?::<type>, ?::<type>" is executed
        # with bound float values [123.456, -789.012, 42.0]
        sql = f"SELECT ?::{float_type}, ?::{float_type}, ?::{float_type}"
        result = execute_query(sql, (123.456, -789.012, 42.0), single_row=True)

        # Then Result should contain floats [123.456, -789.012, 42.0]
        assert_floats_equal(result, [123.456, -789.012, 42.0])
        assert_type(result, float)

    @float_type_parametrize
    def test_should_select_null_float_using_parameter_binding_for_float_and_synonyms(self, execute_query, float_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::<type>" is executed with bound NULL value
        sql = f"SELECT ?::{float_type}"
        result = execute_query(sql, (None,), single_row=True)

        # Then Result should contain NULL
        assert_floats_equal(result, [None])

    @float_type_parametrize
    def test_should_insert_float_using_parameter_binding_for_float_and_synonyms(
        self, execute_query, executemany_insert, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.float_bind_table_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")

        # When Float values [0.0, 123.456, -789.012, NULL] are bulk-inserted using multirow binding

        # Note: NaN, inf, -inf cannot be bound — Snowflake rejects them as bind values.
        test_rows = [(0.0,), (123.456,), (-789.012,), (None,)]
        rows = executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_rows)

        # Then Result should contain the same values including NULL
        result = [row[0] for row in rows]
        assert len(result) == len(test_rows)
        assert_type(result, float, can_be_none=True)
        # Compare as sets (order depends on ORDER BY in executemany_insert fixture)
        non_null_result = {v for v in result if v is not None}
        non_null_expected = {0.0, 123.456, -789.012}
        assert non_null_result == non_null_expected
        assert result.count(None) == 1
