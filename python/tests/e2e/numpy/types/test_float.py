"""FLOAT type tests for Universal Driver -- numpy consumer.

Arrow float64 -> pandas float64 for all FLOAT synonyms.
NULL -> NaN. Special values (NaN, inf, -inf) are preserved.
"""

from __future__ import annotations

from math import inf, isnan, nan

import pytest

from tests.conftest import with_paramstyle
from tests.e2e.numpy.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_float,
)
from tests.e2e.types.utils import assert_sequential_values, floats_equal


# Test constants ported from tests/e2e/types/test_float.py
FLOAT_TYPE_SYNONYMS = ["FLOAT", "FLOAT4", "FLOAT8", "DOUBLE", "DOUBLE PRECISION", "REAL"]
float_type_parametrize = pytest.mark.parametrize("float_type", FLOAT_TYPE_SYNONYMS)

FLOAT_MAX = 1.7976931348623157e308
FLOAT_MIN = -1.7976931348623157e308
FLOAT_MIN_NORMAL = 2.2250738585072014e-308
FLOAT_MIN_SUBNORMAL = 5e-324
FLOAT_15_DIGITS = 123456789012345.0
FLOAT_16_DIGITS = 1234567890123456.0
FLOAT_REALISTIC_MAX = 1.79769313486231e308
LARGE_RESULT_SET_SIZE = 50_000


class TestFetchNumpyFloatTypeCasting:
    """Type-casting coverage for the FLOAT family via numpy fetch."""

    @float_type_parametrize
    def test_should_cast_float_values_to_appropriate_type_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 0.0::<type>, 123.456::<type>, 1.23e10::<type>,
        # 'NaN'::<type>, 'inf'::<type>" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT 0.0::{float_type}, 123.456::{float_type}, 1.23e10::{float_type}, "
            f"'NaN'::{float_type}, 'inf'::{float_type}",
        )

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_float, is_float, is_float, is_float, is_float])

        # And Regular values should have approximately 15 decimal digits precision
        row = get_row(df, 0)
        assert row[:3] == pytest.approx([0.0, 123.456, 1.23e10])

        # And NaN and inf values should be identified correctly
        assert row == pytest.approx([0.0, 123.456, 1.23e10, nan, inf], nan_ok=True)


class TestFetchNumpyFloatLiteral:
    """SELECT-with-literal coverage via numpy fetch."""

    @float_type_parametrize
    def test_should_select_float_literals_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>,
        # 123.456::<type>, -123.456::<type>" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT 0.0::{float_type}, 1.0::{float_type}, -1.0::{float_type}, "
            f"123.456::{float_type}, -123.456::{float_type}",
        )

        # Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]
        assert_dtypes(df, [is_float, is_float, is_float, is_float, is_float])
        assert get_row(df, 0) == pytest.approx([0.0, 1.0, -1.0, 123.456, -123.456])

    @float_type_parametrize
    def test_should_handle_special_float_values_from_literals_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
        df = execute_and_fetch(cursor, f"SELECT 'NaN'::{float_type}, 'inf'::{float_type}, '-inf'::{float_type}")

        # Then Result should contain [NaN, positive_infinity, negative_infinity]
        assert_dtypes(df, [is_float, is_float, is_float])
        assert get_row(df, 0) == pytest.approx([nan, inf, -inf], nan_ok=True)

    BOUNDARY_LITERAL_CASES = [
        ((FLOAT_MAX, FLOAT_MIN), [FLOAT_MAX, FLOAT_MIN]),
        ((FLOAT_MIN_NORMAL, FLOAT_MIN_SUBNORMAL), [FLOAT_MIN_NORMAL, FLOAT_MIN_SUBNORMAL]),
    ]

    @float_type_parametrize
    @pytest.mark.parametrize(
        "select_values,expected",
        BOUNDARY_LITERAL_CASES,
        ids=["max", "min"],
    )
    @pytest.mark.skip_for_json_result_set(reason="JSON format loses precision for Double.MAX_VALUE boundary values")
    def test_should_handle_float_case_boundary_values_from_literals_for_float_and_synonyms(
        self, cursor, float_type, select_values, expected
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT <query_values>" is executed
        columns = ", ".join(f"{v}::{float_type}" for v in select_values)
        df = execute_and_fetch(cursor, f"SELECT {columns}")

        # Then Result should contain floats [<expected_values>]
        assert_dtypes(df, [is_float, is_float])
        assert get_row(df, 0) == pytest.approx(expected)

    @float_type_parametrize
    def test_should_handle_realistic_large_float_case_boundary_values_from_literals_for_float_and_synonyms(
        self, cursor, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT <query_values>" is executed
        df = execute_and_fetch(
            cursor, f"SELECT {FLOAT_REALISTIC_MAX}::{float_type}, -{FLOAT_REALISTIC_MAX}::{float_type}"
        )

        # Then Result should contain floats [<expected_values>]
        assert_dtypes(df, [is_float, is_float])
        assert get_row(df, 0) == pytest.approx([FLOAT_REALISTIC_MAX, -FLOAT_REALISTIC_MAX])

    @float_type_parametrize
    def test_should_handle_float_precision_boundary_values_from_literals_for_float_and_synonyms(
        self, cursor, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
        df = execute_and_fetch(cursor, f"SELECT {FLOAT_15_DIGITS}::{float_type}, {FLOAT_16_DIGITS}::{float_type}")

        # Then Result should verify precision around 15 decimal digits
        assert_dtypes(df, [is_float, is_float])
        assert get_row(df, 0) == pytest.approx([FLOAT_15_DIGITS, FLOAT_16_DIGITS])

    @float_type_parametrize
    def test_should_handle_null_values_from_literals_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
        df = execute_and_fetch(cursor, f"SELECT NULL::{float_type}, 42.5::{float_type}, NULL::{float_type}")

        # Then Result should contain [NULL, 42.5, NULL]
        assert_dtypes(df, [is_float, is_float, is_float])
        assert get_row(df, 0)[0] is None
        assert get_row(df, 0)[1] == pytest.approx(42.5)
        assert get_row(df, 0)[2] is None

    @float_type_parametrize
    def test_should_download_large_result_set_with_multiple_chunks_from_generator_for_float_and_synonyms(
        self, cursor, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq4()) - 1)::{float_type} as id "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY 1",
        )

        # Then Result should contain 50000 rows with all values returned as appropriate float type
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=float, compare=floats_equal)


class TestFetchNumpyFloatTable:
    """Table-based scenarios via numpy fetch."""

    @float_type_parametrize
    def test_should_select_floats_from_table_for_float_and_synonyms(
        self, execute_query, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
        table_name = f"{tmp_schema}.np_float_table_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        for v in [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]:
            execute_query(f"INSERT INTO {table_name} VALUES ({v})")

        # When Query "SELECT * FROM float_table" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]
        assert_dtypes(df, [is_float])
        assert get_column(df, 0) == pytest.approx([-789.012, -9.87e-3, 0.0, 123.456, 123000.0])

    @float_type_parametrize
    def test_should_handle_special_float_values_from_table_for_float_and_synonyms(
        self, execute_query, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with <type> column exists with values [NaN, inf, -inf, 42.0, -42.0]
        table_name = f"{tmp_schema}.np_special_float_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        execute_query(
            f"INSERT INTO {table_name} VALUES "
            f"('NaN'::{float_type}), ('inf'::{float_type}), ('-inf'::{float_type}), "
            f"(42.0::{float_type}), (-42.0::{float_type})"
        )

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0]
        assert_dtypes(df, [is_float])
        col = get_column(df, 0)
        assert col[0] == pytest.approx(-inf)
        assert col[1] == pytest.approx(-42.0)
        assert col[2] == pytest.approx(42.0)
        assert col[3] == pytest.approx(inf)
        assert col[4] is None or col[4] != col[4]

    @float_type_parametrize
    @pytest.mark.skip_for_json_result_set(reason="JSON format loses precision for 64-bit FLOAT max boundary values")
    def test_should_handle_float_boundary_values_from_table_for_float_and_synonyms(
        self, execute_query, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with <type> column exists with boundary values
        # [1.7976931348623157e308, -1.7976931348623157e308, 2.2250738585072014e-308, 5e-324, 123456789012345.0]
        table_name = f"{tmp_schema}.np_boundary_float_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        boundary_values = [FLOAT_MAX, FLOAT_MIN, FLOAT_MIN_NORMAL, FLOAT_MIN_SUBNORMAL, FLOAT_15_DIGITS]
        for val in boundary_values:
            execute_query(f"INSERT INTO {table_name} VALUES ({val})")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain maximum, minimum, and precision boundary values
        # preserved within float precision limits
        assert_dtypes(df, [is_float])
        col = get_column(df, 0)
        assert col == pytest.approx(sorted(boundary_values))

    @float_type_parametrize
    def test_should_handle_null_values_from_table_for_float_and_synonyms(
        self, execute_query, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
        table_name = f"{tmp_schema}.np_null_float_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        execute_query(f"INSERT INTO {table_name} VALUES (NULL), (123.456), (NULL), (-789.012)")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain [NULL, 123.456, NULL, -789.012]
        assert_dtypes(df, [is_float])
        col = get_column(df, 0)
        assert col[:2] == pytest.approx([-789.012, 123.456])
        assert col[2] is None
        assert col[3] is None

    @float_type_parametrize
    def test_should_select_large_result_set_from_table_for_float_and_synonyms(
        self, execute_query, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with <type> column exists with 50000 sequential values
        table_name = f"{tmp_schema}.np_large_float_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq4()) - 1)::{float_type} "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table>" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 50000 rows with all values returned as appropriate float type
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=float, compare=floats_equal)


@with_paramstyle("qmark")
class TestFetchNumpyFloatBinding:
    """Parameter-binding scenarios via numpy fetch."""

    @float_type_parametrize
    def test_should_select_float_using_parameter_binding_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT ?::<type>, ?::<type>, ?::<type>" is executed
        # with bound float values [123.456, -789.012, 42.0]
        df = execute_and_fetch(
            cursor,
            f"SELECT ?::{float_type}, ?::{float_type}, ?::{float_type}",
            params=(123.456, -789.012, 42.0),
        )

        # Then Result should contain floats [123.456, -789.012, 42.0]
        assert_dtypes(df, [is_float, is_float, is_float])
        assert get_row(df, 0) == pytest.approx([123.456, -789.012, 42.0])

    @float_type_parametrize
    def test_should_select_null_float_using_parameter_binding_for_float_and_synonyms(self, cursor, float_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT ?::<type>" is executed with bound NULL value
        df = execute_and_fetch(cursor, f"SELECT ?::{float_type}", params=(None,))

        # Then Result should contain NULL
        assert_dtypes(df, [is_float])
        assert get_row(df, 0)[0] is None

    @float_type_parametrize
    def test_should_insert_float_using_parameter_binding_for_float_and_synonyms(
        self, execute_query, executemany_insert, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.np_float_bind_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")

        # When Float values [0.0, 123.456, -789.012, NULL] are bulk-inserted using multirow binding
        test_data = [(0.0,), (123.456,), (-789.012,), (None,)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_data)

        # Then Result should contain the same values including NULL
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")
        assert_dtypes(df, [is_float])
        col = get_column(df, 0)
        assert col[:3] == pytest.approx([-789.012, 0.0, 123.456])
        assert col[3] is None

    @float_type_parametrize
    def test_should_insert_non_finite_float_values_using_parameter_binding_for_float_and_synonyms(
        self, execute_query, executemany_insert, cursor, tmp_schema, float_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.np_float_bind_non_finite_{float_type.replace(' ', '_').lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {float_type})")

        # When Float values [0.0, 123.456, -789.012, NULL, NaN, inf, -inf] are bulk-inserted
        # using multirow binding
        test_data = [(0.0,), (123.456,), (-789.012,), (None,), (nan,), (inf,), (-inf,)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_data)

        # Then Result should contain the same values including NULL and the special values
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")
        assert_dtypes(df, [is_float])
        col = get_column(df, 0)
        assert len(col) == len(test_data)
        assert col[:5] == pytest.approx([-inf, -789.012, 0.0, 123.456, inf])
        assert sum(1 for v in col[5:] if v is None) == 1
        assert sum(1 for v in col[5:] if v is not None and isnan(v)) == 1
