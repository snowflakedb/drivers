"""VECTOR type tests for Universal Driver.

This module tests the VECTOR type which stores fixed-size arrays of numeric values.
Subtypes: INT (integer) and FLOAT (32-bit floating-point).
Values are returned as Python lists (list[int] or list[float]).
Maximum dimension: 4096.

Reference: https://docs.snowflake.com/en/sql-reference/data-types-vector
"""

from __future__ import annotations

import pytest

from .utils import assert_type


# =============================================================================
# INT VECTOR TEST VALUES
# =============================================================================
INT_VEC_3D = [1, 2, 3]
INT_VEC_3D_B = [10, 20, 30]

# =============================================================================
# FLOAT VECTOR TEST VALUES
# =============================================================================
FLOAT_VEC_3D = [1.5, 2.5, 3.5]
FLOAT_VEC_5D = [1.1, 2.2, 3.3, 4.4, 5.5]
FLOAT_VEC_5D_B = [10.5, 20.5, 30.5, 40.5, 50.5]

# =============================================================================
# LARGE DIMENSION TEST
# =============================================================================
LARGE_DIMENSION_SIZE = 256

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 20_000


def _vec_sql(values: list, subtype: str) -> str:
    """Build a VECTOR SQL literal, e.g. ``[1, 2, 3]::VECTOR(INT, 3)``."""
    return f"{values}::VECTOR({subtype}, {len(values)})"


class TestVectorTypeCasting:
    """Tests for VECTOR type casting to appropriate type."""

    def test_should_cast_vector_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
        sql = f"SELECT {_vec_sql(INT_VEC_3D, 'INT')}, {_vec_sql(FLOAT_VEC_3D, 'FLOAT')}"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, list)
        assert result[0] == INT_VEC_3D
        assert result[1] == pytest.approx(FLOAT_VEC_3D)


class TestVectorLiteral:
    """Tests for VECTOR type using SELECT with literals (no tables)."""

    @pytest.mark.parametrize(
        "query_value, expected_value, is_float",
        [
            ("[1, 3, -5]::VECTOR(INT, 3)", [1, 3, -5], False),
            ("[40, 1234567]::VECTOR(INT, 2)", [40, 1234567], False),
            ("[1.8, -3.4, 6.7, 0, 2.3]::VECTOR(FLOAT, 5)", [1.8, -3.4, 6.7, 0.0, 2.3], True),
        ],
        ids=["INT-3d", "INT-2d", "FLOAT-5d"],
    )
    def test_should_select_subtype_vector_literal(self, execute_query, query_value, expected_value, is_float):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_value>" is executed
        sql = f"SELECT {query_value}"
        result = execute_query(sql, single_row=True)

        # Then Result should contain <subtype> vector <expected_value>
        assert isinstance(result[0], list)
        if is_float:
            assert result[0] == pytest.approx(expected_value)
        else:
            assert result[0] == expected_value

    def test_should_select_large_dimension_vector(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query generating a 256-dimension FLOAT vector is executed
        expected = [float(i) for i in range(LARGE_DIMENSION_SIZE)]
        values = ", ".join(str(v) for v in expected)
        sql = f"SELECT [{values}]::VECTOR(FLOAT, {LARGE_DIMENSION_SIZE})"
        result = execute_query(sql, single_row=True)

        # Then Result should contain a 256-element float vector
        assert isinstance(result[0], list)
        assert len(result[0]) == LARGE_DIMENSION_SIZE
        assert result[0] == pytest.approx(expected)

    def test_should_handle_null_vector_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
        sql = f"SELECT {_vec_sql(INT_VEC_3D, 'INT')}, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [[1, 2, 3], NULL, NULL]
        assert result[0] == INT_VEC_3D
        assert result[1] is None
        assert result[2] is None


class TestVectorTable:
    """Tests for VECTOR type using table operations."""

    def test_should_select_vector_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
        table_name = f"{tmp_schema}.vector_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} "
            f"(id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 5))"
        )
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT 1, {_vec_sql(INT_VEC_3D, 'INT')}, {_vec_sql(FLOAT_VEC_5D, 'FLOAT')} "
            f"UNION ALL SELECT 2, {_vec_sql(INT_VEC_3D_B, 'INT')}, {_vec_sql(FLOAT_VEC_5D_B, 'FLOAT')}"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT int_vec, float_vec FROM {table_name} ORDER BY id")

        # Then Result should contain the expected integer and float vector values
        assert len(rows) == 2
        assert rows[0][0] == INT_VEC_3D
        assert rows[0][1] == pytest.approx(FLOAT_VEC_5D)
        assert rows[1][0] == INT_VEC_3D_B
        assert rows[1][1] == pytest.approx(FLOAT_VEC_5D_B)

    def test_should_handle_null_vector_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with VECTOR columns exists containing NULLs and values
        table_name = f"{tmp_schema}.vector_null_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} "
            f"(id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 3))"
        )
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT 1, {_vec_sql(INT_VEC_3D, 'INT')}, NULL::VECTOR(FLOAT, 3) "
            f"UNION ALL SELECT 2, NULL::VECTOR(INT, 3), {_vec_sql(FLOAT_VEC_3D, 'FLOAT')} "
            f"UNION ALL SELECT 3, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT int_vec, float_vec FROM {table_name} ORDER BY id")

        # Then Result should contain both vector values and NULLs
        assert len(rows) == 3
        assert rows[0][0] == INT_VEC_3D
        assert rows[0][1] is None
        assert rows[1][0] is None
        assert rows[1][1] == pytest.approx(FLOAT_VEC_3D)
        assert rows[2][0] is None
        assert rows[2][1] is None


class TestVectorMultipleChunks:
    """Tests for VECTOR type with multiple chunks downloading."""

    @pytest.mark.skip_for_json_result_set(
        reason="Multichunk vector generates dynamic data that may not round-trip identically in JSON format"
    )
    def test_should_download_vector_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query generating 20000 integer vectors is executed
        sql = (
            f"SELECT [seq8(), seq8() * 2, seq8() * 3]::VECTOR(INT, 3) AS vec "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        )
        rows = execute_query(sql)

        # Then All 20000 rows should be fetched and each should be a non-null list value
        assert len(rows) == LARGE_RESULT_SET_SIZE
        for row in rows:
            assert isinstance(row[0], list)
