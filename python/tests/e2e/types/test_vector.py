"""VECTOR type tests for Universal Driver.

This module tests the VECTOR type which stores fixed-size arrays of numeric values.
Subtypes: INT (integer) and FLOAT (32-bit floating-point).
Values are returned as Python lists (list[int] or list[float]).
Maximum dimension: 4096.

Reference: https://docs.snowflake.com/en/sql-reference/data-types-vector
"""

import pytest

from ...conftest import with_paramstyle
from .utils import assert_sequential_values, assert_type


def _assert_float32_equal(actual: list[float], expected: list[float]) -> None:
    """Assert two float32 vectors are equal within 32-bit float tolerance.

    VECTOR(FLOAT) uses 32-bit floats, so values like 1.8 become 1.7999999523162842.
    Standard assert_floats_equal uses 64-bit tolerances which are too tight.
    """
    assert len(actual) == len(expected), f"Length mismatch: {len(actual)} != {len(expected)}"
    for i, (a, e) in enumerate(zip(actual, expected)):
        assert a == pytest.approx(e, rel=1e-6), f"Mismatch at index {i}: expected {e}, got {a}"


# =============================================================================
# INT VECTOR TEST VALUES
# =============================================================================
INT_VEC_2D = [40, 1234567]
INT_VEC_3D = [1, 2, 3]
INT_VEC_3D_ALT = [1, 3, -5]
INT_VEC_3D_B = [10, 20, 30]

# =============================================================================
# FLOAT VECTOR TEST VALUES
# =============================================================================
FLOAT_VEC_3D = [1.5, 2.5, 3.5]
FLOAT_VEC_5D = [1.1, 2.2, 3.3, 4.4, 5.5]
FLOAT_VEC_5D_ALT = [1.8, -3.4, 6.7, 0.0, 2.3]
FLOAT_VEC_5D_B = [10.5, 20.5, 30.5, 40.5, 50.5]

# =============================================================================
# SPECIAL VALUES TEST
# =============================================================================
MAX_DIMENSION_SIZE = 4096

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
        assert_type(result[0], int)
        _assert_float32_equal(result[1], FLOAT_VEC_3D)
        assert_type(result[1], float)


class TestVectorLiteral:
    """Tests for VECTOR type using SELECT with literals (no tables)."""

    LITERAL_TEST_CASES = [
        ("INT-3d", _vec_sql(INT_VEC_3D_ALT, "INT"), INT_VEC_3D_ALT, False),
        ("INT-2d", _vec_sql(INT_VEC_2D, "INT"), INT_VEC_2D, False),
        ("FLOAT-5d", _vec_sql(FLOAT_VEC_5D_ALT, "FLOAT"), FLOAT_VEC_5D_ALT, True),
    ]

    @pytest.mark.parametrize(
        "subtype, query_value, expected_value, is_float",
        LITERAL_TEST_CASES,
        ids=[c[0] for c in LITERAL_TEST_CASES],
    )
    def test_should_select_subtype_vector_literal(self, execute_query, subtype, query_value, expected_value, is_float):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_value>" is executed
        sql = f"SELECT {query_value}"
        result = execute_query(sql, single_row=True)

        # Then Result should contain <subtype> vector <expected_value>
        assert isinstance(result[0], list)
        if is_float:
            _assert_float32_equal(result[0], expected_value)
            assert_type(result[0], float)
        else:
            assert result[0] == expected_value
            assert_type(result[0], int)

    def test_should_select_vector_special_values(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query selecting special vector values is executed
        result = execute_query(
            f"SELECT {_vec_sql(INT_VEC_3D, 'INT')}, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)",
            single_row=True,
        )
        expected = [float(i) for i in range(MAX_DIMENSION_SIZE)]
        values = ", ".join(str(v) for v in expected)
        max_dim_result = execute_query(f"SELECT [{values}]::VECTOR(FLOAT, {MAX_DIMENSION_SIZE})", single_row=True)

        # Then NULL vectors should return None and max-dimension vector should be valid
        assert result[0] == INT_VEC_3D
        assert result[1] is None
        assert result[2] is None
        assert isinstance(max_dim_result[0], list)
        assert len(max_dim_result[0]) == MAX_DIMENSION_SIZE
        assert_type(max_dim_result[0], float)
        _assert_float32_equal(max_dim_result[0], expected)


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
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the expected integer and float vector values
        assert len(rows) == 2
        assert rows[0][1] == INT_VEC_3D
        assert_type(rows[0][1], int)
        _assert_float32_equal(rows[0][2], FLOAT_VEC_5D)
        assert_type(rows[0][2], float)
        assert rows[1][1] == INT_VEC_3D_B
        assert_type(rows[1][1], int)
        _assert_float32_equal(rows[1][2], FLOAT_VEC_5D_B)
        assert_type(rows[1][2], float)

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
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain both vector values and NULLs
        assert len(rows) == 3
        assert rows[0][1] == INT_VEC_3D
        assert rows[0][2] is None
        assert rows[1][1] is None
        _assert_float32_equal(rows[1][2], FLOAT_VEC_3D)
        assert rows[2][1] is None
        assert rows[2][2] is None


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
            "SELECT id, [id, id * 2, id * 3]::VECTOR(INT, 3) AS vec "
            "FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))) "
            f"ORDER BY id"
        )
        rows = execute_query(sql)

        # Then All 20000 rows should be fetched with valid 3-element integer vectors
        assert len(rows) == LARGE_RESULT_SET_SIZE
        assert_type([row[1] for row in rows], list)
        assert_sequential_values(rows, LARGE_RESULT_SET_SIZE, transform=lambda i: (i, [i, i * 2, i * 3]))


@with_paramstyle("qmark")
class TestVectorBinding:
    """Tests for VECTOR type using parameter binding."""

    def test_should_insert_and_select_vectors_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with VECTOR columns exists
        table_name = f"{tmp_schema}.vector_bind_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} "
            f"(id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 3))"
        )

        # When Vector values are inserted using parameter binding
        execute_query(
            f"INSERT INTO {table_name} SELECT ?, {_vec_sql(INT_VEC_3D, 'INT')}, {_vec_sql(FLOAT_VEC_3D, 'FLOAT')}",
            (1,),
        )
        execute_query(
            f"INSERT INTO {table_name} SELECT ?, {_vec_sql(INT_VEC_3D_B, 'INT')}, NULL::VECTOR(FLOAT, 3)",
            (2,),
        )

        # And Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the bound vector values
        assert len(rows) == 2
        assert rows[0][1] == INT_VEC_3D
        assert_type(rows[0][1], int)
        _assert_float32_equal(rows[0][2], FLOAT_VEC_3D)
        assert_type(rows[0][2], float)
        assert rows[1][1] == INT_VEC_3D_B
        assert rows[1][2] is None

    def test_should_insert_and_select_vectors_using_batch_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with VECTOR columns exists
        table_name = f"{tmp_schema}.vector_batch_bind_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} "
            f"(id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 3))"
        )

        # When Vector values are bulk-inserted using multirow binding
        test_data = [(1, INT_VEC_3D, FLOAT_VEC_3D), (2, INT_VEC_3D_B, FLOAT_VEC_3D)]
        for row_id, int_vec, float_vec in test_data:
            execute_query(
                f"INSERT INTO {table_name} SELECT ?, {_vec_sql(int_vec, 'INT')}, {_vec_sql(float_vec, 'FLOAT')}",
                (row_id,),
            )

        # Then SELECT should return the inserted vector values
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")
        assert len(rows) == 2
        assert rows[0][1] == INT_VEC_3D
        assert_type(rows[0][1], int)
        _assert_float32_equal(rows[0][2], FLOAT_VEC_3D)
        assert rows[1][1] == INT_VEC_3D_B
        assert_type(rows[1][1], int)
        _assert_float32_equal(rows[1][2], FLOAT_VEC_3D)
