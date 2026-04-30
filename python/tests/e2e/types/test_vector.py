"""VECTOR type tests for Universal Driver.

This module tests the VECTOR type which stores fixed-size arrays of numeric values.
Subtypes: INT (integer) and FLOAT (32-bit floating-point).
Values are returned as Python lists (list[int] or list[float]).
Maximum dimension: 4096.

Reference: https://docs.snowflake.com/en/sql-reference/data-types-vector
"""

import pytest

from .utils import assert_floats_equal, assert_sequential_values, assert_type


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

SKIP_JSON = pytest.mark.skip_for_json_result_set(reason="VECTOR type is not supported in JSON result format")


@SKIP_JSON
class TestVectorTypeCasting:
    """Tests for VECTOR type casting to appropriate type."""

    def test_should_cast_vector_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
        sql = f"SELECT {INT_VEC_3D}::VECTOR(INT, 3), {FLOAT_VEC_3D}::VECTOR(FLOAT, 3)"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, list)
        assert result[0] == INT_VEC_3D
        assert_type(result[0], int)
        assert_floats_equal(result[1], FLOAT_VEC_3D)
        assert_type(result[1], float)


@SKIP_JSON
class TestVectorLiteral:
    """Tests for VECTOR type using SELECT with literals (no tables)."""

    LITERAL_TEST_CASES = [
        ("INT-3d", "INT", INT_VEC_3D_ALT),
        ("INT-2d", "INT", INT_VEC_2D),
        ("FLOAT-5d", "FLOAT", FLOAT_VEC_5D_ALT),
    ]

    @pytest.mark.parametrize(
        "subtype, vec_type, expected_value",
        LITERAL_TEST_CASES,
        ids=[c[0] for c in LITERAL_TEST_CASES],
    )
    def test_should_select_subtype_vector_literal(self, execute_query, subtype, vec_type, expected_value):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
        sql = f"SELECT {expected_value}::VECTOR({vec_type}, {len(expected_value)})"
        result = execute_query(sql, single_row=True)

        # Then Result should contain <subtype> vector <expected_value>
        assert isinstance(result[0], list)
        assert len(result[0]) == len(expected_value)
        if vec_type == "FLOAT":
            assert_floats_equal(result[0], expected_value)
            assert_type(result[0], float)
        else:
            assert result[0] == expected_value
            assert_type(result[0], int)

    def test_should_handle_null_vector_values(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
        result = execute_query(
            f"SELECT {INT_VEC_3D}::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)",
            single_row=True,
        )

        # Then Result should contain [[1, 2, 3], NULL, NULL]
        assert result[0] == INT_VEC_3D
        assert result[1] is None
        assert result[2] is None

    def test_should_select_max_dimension_vector(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query selecting 4096-element float vector is executed
        expected = [float(i) for i in range(MAX_DIMENSION_SIZE)]
        values = ", ".join(str(v) for v in expected)
        result = execute_query(f"SELECT [{values}]::VECTOR(FLOAT, {MAX_DIMENSION_SIZE})", single_row=True)

        # Then Result should be a valid 4096-element float vector
        assert isinstance(result[0], list)
        assert len(result[0]) == MAX_DIMENSION_SIZE
        assert_type(result[0], float)
        assert_floats_equal(result[0], expected)


@SKIP_JSON
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
            f"SELECT 1, {INT_VEC_3D}::VECTOR(INT, 3), {FLOAT_VEC_5D}::VECTOR(FLOAT, 5) "
            f"UNION ALL SELECT 2, {INT_VEC_3D_B}::VECTOR(INT, 3), {FLOAT_VEC_5D_B}::VECTOR(FLOAT, 5)"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the expected integer and float vector values
        assert len(rows) == 2
        assert rows[0][1] == INT_VEC_3D
        assert_type(rows[0][1], int)
        assert_floats_equal(rows[0][2], FLOAT_VEC_5D)
        assert_type(rows[0][2], float)
        assert rows[1][1] == INT_VEC_3D_B
        assert_type(rows[1][1], int)
        assert_floats_equal(rows[1][2], FLOAT_VEC_5D_B)
        assert_type(rows[1][2], float)

    def test_should_handle_null_vector_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass
        # And Table with VECTOR columns exist containing NULLs and values
        table_name = f"{tmp_schema}.vector_null_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} "
            f"(id INT, int_vec VECTOR(INT, 3), float_vec VECTOR(FLOAT, 3))"
        )
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT 1, {INT_VEC_3D}::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3) "
            f"UNION ALL SELECT 2, NULL::VECTOR(INT, 3), {FLOAT_VEC_3D}::VECTOR(FLOAT, 3) "
            f"UNION ALL SELECT 3, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain both vector values and NULLs
        assert len(rows) == 3
        assert rows[0][1] == INT_VEC_3D
        assert rows[0][2] is None
        assert rows[1][1] is None
        assert_floats_equal(rows[1][2], FLOAT_VEC_3D)
        assert rows[2][1] is None
        assert rows[2][2] is None


@SKIP_JSON
class TestVectorMultipleChunks:
    """Tests for VECTOR type with multiple chunks downloading."""

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
