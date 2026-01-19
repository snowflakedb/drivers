"""FLOAT type tests for Universal Driver.

This module tests FLOAT type and its synonyms (FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL)
across various scenarios including literals, table operations, special values (NaN, infinity),
boundary values, NULL handling, parameter binding, large result sets, and type casting.

All tests are parameterized to run with each type synonym to verify they behave identically.
All type synonyms are treated as 64-bit IEEE 754 double precision.
"""

import math

import pytest


FLOAT_TYPE_SYNONYMS = [
    "FLOAT",
    "FLOAT4",
    "FLOAT8",
    "DOUBLE",
    "DOUBLE PRECISION",
    "REAL",
]
float_type_parametrize = pytest.mark.parametrize("float_type", FLOAT_TYPE_SYNONYMS)


class TestFloat:
    """Test suite for FLOAT type and synonyms."""

    @float_type_parametrize
    def test_should_select_float_literals_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>, 123.456::<type>, -123.456::<type>" is executed
        sql = (
            f"SELECT 0.0::{float_type}, 1.0::{float_type}, -1.0::{float_type}, "
            f"123.456::{float_type}, -123.456::{float_type}"
        )
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]
        expected = (0.0, 1.0, -1.0, 123.456, -123.456)
        assert len(result) == len(expected), f"Expected {len(expected)} values, got {len(result)}"
        for actual, expect in zip(result, expected):
            assert abs(actual - expect) < 1e-10, f"Expected {expect}, got {actual}"
            assert isinstance(actual, float), f"Value {actual} should be Python float type"

    @float_type_parametrize
    def test_should_select_floats_from_table_for_float_and_synonyms(self, cursor, tmp_schema, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
        table_name = f"{tmp_schema}.float_table_{float_type.replace(' ', '_').lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {float_type})")
        test_values = [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
        for val in test_values:
            cursor.execute(f"INSERT INTO {table_name} VALUES ({val})")

        # When Query "SELECT * FROM float_table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()

        # Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]
        result = [row[0] for row in rows]
        expected = [0.0, 123.456, -789.012, 123000.0, -0.00987]

        for actual, expect in zip(result, expected):
            assert abs(actual - expect) < 1e-10, f"Expected {expect}, got {actual}"
            assert isinstance(actual, float), f"Value {actual} should be Python float type"

    @float_type_parametrize
    def test_should_handle_special_float_values_from_literals_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
        sql = f"SELECT 'NaN'::{float_type}, 'inf'::{float_type}, '-inf'::{float_type}"
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then Result should contain [NaN, positive_infinity, negative_infinity]
        assert math.isnan(result[0]), "First value should be NaN"
        assert result[1:] == (
            float("inf"),
            float("-inf"),
        ), "Remaining values should be inf, -inf"
        assert all(isinstance(val, float) for val in result), "All values should be Python float type"

    @float_type_parametrize
    def test_should_handle_special_float_values_from_table_for_float_and_synonyms(self, cursor, tmp_schema, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists with values [NaN, inf, -inf, 42.0, -42.0]
        table_name = f"{tmp_schema}.special_float_table_{float_type.replace(' ', '_').lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {float_type})")
        cursor.execute(
            f"INSERT INTO {table_name} VALUES\n"
            f"('NaN'::{float_type}),\n"
            f"('inf'::{float_type}),\n"
            f"('-inf'::{float_type}),\n"
            f"(42.0::{float_type}),\n"
            f"(-42.0::{float_type})"
        )

        # When Query "SELECT * FROM special_float_table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()
        values = [row[0] for row in rows]

        # Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0]
        assert all(isinstance(v, float) for v in values), "All values should be floats"
        assert math.isnan(values[0]), "First value should be NaN"
        assert values[1:] == [
            float("inf"),
            float("-inf"),
            42.0,
            -42.0,
        ], "Remaining values should be inf, -inf, 42.0, -42.0"

    @float_type_parametrize
    def test_should_handle_float_boundary_values_from_literals_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 1.7976931348623157e308::<type>, -1.7976931348623157e308::<type>" is executed
        sql = f"SELECT 1.7976931348623157e308::{float_type}, -1.7976931348623157e308::{float_type}"
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then Result should contain floats [1.7976931348623157e308, -1.7976931348623157e308]
        assert abs(result[0] - 1.7976931348623157e308) <= abs(1.7976931348623157e308) * 1e-14
        assert abs(result[1] - (-1.7976931348623157e308)) <= abs(-1.7976931348623157e308) * 1e-14

        # When Query "SELECT 2.2250738585072014e-308::<type>, 5e-324::<type>" is executed
        sql = f"SELECT 2.2250738585072014e-308::{float_type}, 5e-324::{float_type}"
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then Result should contain floats [2.2250738585072014e-308, approximately 5e-324]
        assert abs(result[0] - 2.2250738585072014e-308) <= 1e-320
        assert abs(result[1] - 5e-324) <= 1e-320  # Subnormal tolerance

        # When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
        sql = f"SELECT 123456789012345.0::{float_type}, 1234567890123456.0::{float_type}"
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then Result should verify precision around 15 decimal digits
        assert abs(result[0] - 123456789012345.0) <= abs(123456789012345.0) * 1e-14
        # 16-digit value may have precision loss, verify it's close
        assert abs(result[1] - 1234567890123456.0) <= abs(1234567890123456.0) * 1e-13

    @float_type_parametrize
    def test_should_handle_float_boundary_values_from_table_for_float_and_synonyms(
        self, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists with boundary values
        # [1.7976931348623157e308, -1.7976931348623157e308, 2.2250738585072014e-308, 5e-324, 123456789012345.0]
        table_name = f"{tmp_schema}.boundary_table_{float_type.replace(' ', '_').lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {float_type})")
        boundary_values = [
            1.7976931348623157e308,
            -1.7976931348623157e308,
            2.2250738585072014e-308,
            5e-324,
            123456789012345.0,
        ]
        for val in boundary_values:
            cursor.execute(f"INSERT INTO {table_name} VALUES ({val})")

        # When Query "SELECT * FROM boundary_table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()
        result = [row[0] for row in rows]

        # Then Result should contain maximum, minimum, and precision boundary values
        assert len(result) == 5, f"Expected 5 values, got {len(result)}"

        # And All values should be preserved within float precision limits
        assert abs(result[0] - 1.7976931348623157e308) <= abs(1.7976931348623157e308) * 1e-14
        assert abs(result[1] - (-1.7976931348623157e308)) <= abs(-1.7976931348623157e308) * 1e-14
        assert abs(result[2] - 2.2250738585072014e-308) <= 1e-320
        assert abs(result[3] - 5e-324) <= 1e-320
        assert abs(result[4] - 123456789012345.0) <= abs(123456789012345.0) * 1e-14

    @float_type_parametrize
    def test_should_handle_null_values_from_literals_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
        sql = f"SELECT NULL::{float_type}, 42.5::{float_type}, NULL::{float_type}"
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then Result should contain [NULL, 42.5, NULL]
        assert result[0] is None, "First value should be NULL (None)"
        assert abs(result[1] - 42.5) < 1e-10, f"Second value should be 42.5, got {result[1]}"
        assert isinstance(result[1], float), "Second value should be Python float"
        assert result[2] is None, "Third value should be NULL (None)"

    @float_type_parametrize
    def test_should_handle_null_values_from_table_for_float_and_synonyms(self, cursor, tmp_schema, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
        table_name = f"{tmp_schema}.null_table_{float_type.replace(' ', '_').lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {float_type})")
        cursor.execute(f"INSERT INTO {table_name} VALUES (NULL), (123.456), (NULL), (-789.012)")

        # When Query "SELECT * FROM null_table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()
        values = [row[0] for row in rows]

        # Then Result should contain 4 values
        assert len(values) == 4, f"Expected 4 values, got {len(values)}"

        # And Two values should be NULL
        null_count = sum(1 for v in values if v is None)
        assert null_count == 2, f"Expected exactly two NULLs, got {null_count}"

        # And Two values should be floats
        float_count = sum(1 for v in values if v is not None and isinstance(v, float))
        assert float_count == 2, f"Expected exactly two floats, got {float_count}"

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @float_type_parametrize
    def test_should_select_float_using_parameter_binding_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT ?::<type>, ?::<type>, ?::<type>" is executed
        # with bound float values [123.456, -789.012, 42.0]
        sql = f"SELECT ?::{float_type}, ?::{float_type}, ?::{float_type}"
        cursor.execute(sql, (123.456, -789.012, 42.0))
        result = cursor.fetchone()

        # Then Result should contain floats [123.456, -789.012, 42.0]
        expected = (123.456, -789.012, 42.0)
        for actual, expect in zip(result, expected):
            assert abs(actual - expect) < 1e-10, f"Expected {expect}, got {actual}"
            assert isinstance(actual, float), f"Value {actual} should be Python float type"

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @float_type_parametrize
    def test_should_insert_float_using_parameter_binding_for_float_and_synonyms(self, cursor, tmp_schema, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.float_bind_table_{float_type.replace(' ', '_').lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {float_type})")

        # When Float values [0.0, 123.456, -789.012, 1.23e10] are inserted using binding
        test_values = [0.0, 123.456, -789.012, 1.23e10]
        for val in test_values:
            cursor.execute(f"INSERT INTO {table_name} VALUES (?)", (val,))

        # And Query "SELECT * FROM float_table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()

        # Then Result should contain floats [0.0, 123.456, -789.012, 1.23e10]
        result = [row[0] for row in rows]
        assert len(result) == len(test_values), f"Expected {len(test_values)} values, got {len(result)}"
        for actual, expect in zip(result, test_values):
            assert abs(actual - expect) <= abs(expect) * 1e-14 + 1e-10, f"Expected {expect}, got {actual}"
            assert isinstance(actual, float), f"Value {actual} should be Python float type"

    @float_type_parametrize
    def test_should_download_large_result_set_with_multiple_chunks_from_generator_for_float_and_synonyms(
        self, cursor, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v" is executed
        sql = f"SELECT seq8()::{float_type} as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id"
        cursor.execute(sql)
        rows = cursor.fetchall()

        # Then Result should contain 1000000 rows
        expected_count = 1000000
        assert len(rows) == expected_count, f"Expected {expected_count} rows, got {len(rows)}"

        # And All values should be returned as appropriate float type
        for i, row in enumerate(rows):
            assert isinstance(row[0], float), f"Value at row {i} should be Python float type"
            assert int(row[0]) == i, f"Expected row {i} to have value {i}, got {row[0]}"

    @float_type_parametrize
    def test_should_select_large_result_set_from_table_for_float_and_synonyms(self, cursor, tmp_schema, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists with 1000000 sequential values
        table_name = f"{tmp_schema}.large_float_table_{float_type.replace(' ', '_').lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {float_type})")
        cursor.execute(
            f"INSERT INTO {table_name} SELECT seq8()::{float_type} FROM TABLE(GENERATOR(ROWCOUNT => 1000000))"
        )

        # When Query "SELECT * FROM large_float_table" is executed
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY col")
        rows = cursor.fetchall()

        # Then Result should contain 1000000 rows
        expected_count = 1000000
        assert len(rows) == expected_count, f"Expected {expected_count} rows, got {len(rows)}"

        # And All values should be returned as appropriate float type
        for i, row in enumerate(rows):
            assert isinstance(row[0], float), f"Value at row {i} should be Python float type"
            assert row[0] == float(i), f"Expected row {i} to have value {float(i)}, got {row[0]}"

    @float_type_parametrize
    def test_should_cast_float_values_to_native_language_float_type_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 0.0::<type>, 123.456::<type>, 1.23e10::<type>, 'NaN'::<type>, 'inf'::<type>" is executed
        sql = (
            f"SELECT 0.0::{float_type}, 123.456::{float_type}, 1.23e10::{float_type}, "
            f"'NaN'::{float_type}, 'inf'::{float_type}"
        )
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then All values should be returned as appropriate float type
        assert all(isinstance(val, float) for val in result), "All values should be Python float type"

        # And Regular values should have approximately 15 decimal digits precision
        assert result[0] == 0.0, "First value should be 0.0"
        assert abs(result[1] - 123.456) < 1e-10, f"Second value should be 123.456, got {result[1]}"
        assert abs(result[2] - 1.23e10) <= abs(1.23e10) * 1e-14, f"Third value should be 1.23e10, got {result[2]}"

        # And NaN and inf values should be identified correctly
        assert math.isnan(result[3]), "Fourth value should be NaN"
        assert result[4] == float("inf"), "Fifth value should be positive infinity"
