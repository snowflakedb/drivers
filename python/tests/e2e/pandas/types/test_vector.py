"""VECTOR type tests for Universal Driver -- pandas consumer.

Mirrors every scenario in ``tests/definitions/shared/types/vector.feature``
using ``cursor.fetch_pandas_all()`` / ``cursor.fetch_pandas_batches()``.

Arrow FixedSizeList -> pandas ``object`` dtype. Cells are numpy arrays
(``int32`` / ``float32``). NULL becomes ``None``.
"""

from __future__ import annotations

import numpy as np
import pytest

from tests.e2e.pandas.utils import (
    assert_dtypes,
    assert_vector_equal,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_object,
)


INT_VEC_2D = [40, 1234567]
INT_VEC_3D = [1, 2, 3]
INT_VEC_3D_ALT = [1, 3, -5]
INT_VEC_3D_B = [10, 20, 30]
FLOAT_VEC_3D = [1.5, 2.5, 3.5]
FLOAT_VEC_5D = [1.1, 2.2, 3.3, 4.4, 5.5]
FLOAT_VEC_5D_ALT = [1.8, -3.4, 6.7, 0.0, 2.3]
FLOAT_VEC_5D_B = [10.5, 20.5, 30.5, 40.5, 50.5]
MAX_DIMENSION_SIZE = 4096
INT32_MIN = -2_147_483_648
INT32_MAX = 2_147_483_647
FLOAT32_MAX = 3.4028235e38
FLOAT32_SMALLEST_NORMAL = 1.1754944e-38
LARGE_RESULT_SET_SIZE = 20_000

INT_VEC_3D_NP = np.array(INT_VEC_3D)
FLOAT_VEC_3D_NP = np.array(FLOAT_VEC_3D, dtype=np.float32)


class TestFetchPandasVectorTypeCasting:
    """Type-casting coverage for VECTOR via fetch_pandas_all."""

    def test_should_cast_vector_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT {INT_VEC_3D}::VECTOR(INT, 3), {FLOAT_VEC_3D}::VECTOR(FLOAT, 3)",
        )

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_object, is_object])
        row = get_row(df, 0)
        assert_vector_equal(row[0], INT_VEC_3D_NP)
        assert_vector_equal(row[1], FLOAT_VEC_3D_NP)


class TestFetchPandasVectorLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

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
    def test_should_select_subtype_vector_literal(self, cursor, subtype, vec_type, expected_value):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT {expected_value}::VECTOR({vec_type}, {len(expected_value)})",
        )

        # Then Result should contain <subtype> vector <expected_value>
        assert_dtypes(df, [is_object])
        expected = np.array(expected_value, dtype=np.float32 if vec_type == "FLOAT" else None)
        assert_vector_equal(get_row(df, 0)[0], expected)

    def test_should_handle_null_vector_values_from_literals(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT {INT_VEC_3D}::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)",
        )

        # Then Result should contain [[1, 2, 3], NULL, NULL]
        row = get_row(df, 0)
        assert_vector_equal(row[0], INT_VEC_3D_NP)
        assert_vector_equal(row[1], None)
        assert_vector_equal(row[2], None)

    BOUNDARY_TEST_CASES = [
        ("INT", "INT", [INT32_MIN, INT32_MAX, 0]),
        ("FLOAT", "FLOAT", [FLOAT32_MAX, -FLOAT32_MAX, 0.0]),
    ]

    @pytest.mark.parametrize(
        "subtype, vec_type, expected_value",
        BOUNDARY_TEST_CASES,
        ids=[c[0] for c in BOUNDARY_TEST_CASES],
    )
    def test_should_select_subtype_vector_boundary_values(self, cursor, subtype, vec_type, expected_value):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT {expected_value}::VECTOR({vec_type}, {len(expected_value)})",
        )

        # Then Result should preserve <subtype> boundary values
        expected = np.array(expected_value, dtype=np.float32 if vec_type == "FLOAT" else None)
        assert_vector_equal(get_row(df, 0)[0], expected)

    @pytest.mark.skip_for_json_result_set(
        reason="Server-side JSON serialization flushes FLOAT32 subnormals to zero; "
        "Arrow format preserves the bit pattern"
    )
    def test_should_preserve_float_smallest_normal(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query selects a VECTOR(FLOAT, ...) containing FLOAT32_SMALLEST_NORMAL
        expected = [FLOAT32_SMALLEST_NORMAL]
        df = execute_and_fetch(cursor, f"SELECT {expected}::VECTOR(FLOAT, {len(expected)})")

        # Then the smallest-normal value must not underflow to zero
        actual = np.asarray(get_row(df, 0)[0], dtype=np.float32)
        assert actual[0] != 0
        assert actual[0] == pytest.approx(FLOAT32_SMALLEST_NORMAL, rel=1e-6, abs=0)

    def test_should_select_max_dimension_vector(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query selecting 4096-element float vector is executed
        expected = [float(i) for i in range(MAX_DIMENSION_SIZE)]
        values = ", ".join(str(v) for v in expected)
        df = execute_and_fetch(cursor, f"SELECT [{values}]::VECTOR(FLOAT, {MAX_DIMENSION_SIZE})")

        # Then Result should be a valid 4096-element float vector
        actual = np.asarray(get_row(df, 0)[0])
        assert len(actual) == MAX_DIMENSION_SIZE
        assert_vector_equal(actual, np.array(expected, dtype=np.float32))


class TestFetchPandasVectorTable:
    """Table-based scenarios via fetch_pandas_all."""

    def test_should_select_vector_values_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
        table_name = f"{tmp_schema}.pd_vector_table"
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
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain the expected integer and float vector values
        assert len(df) == 2
        assert_vector_equal(get_row(df, 0)[1], INT_VEC_3D)
        assert_vector_equal(get_row(df, 0)[2], np.array(FLOAT_VEC_5D, dtype=np.float32))
        assert_vector_equal(get_row(df, 1)[1], INT_VEC_3D_B)
        assert_vector_equal(get_row(df, 1)[2], np.array(FLOAT_VEC_5D_B, dtype=np.float32))

    def test_should_handle_null_vector_values_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VECTOR columns exist containing NULLs and values
        table_name = f"{tmp_schema}.pd_vector_null_table"
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
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY id")

        # Then Result should contain both vector values and NULLs
        assert len(df) == 3
        assert_vector_equal(get_row(df, 0)[1], INT_VEC_3D)
        assert_vector_equal(get_row(df, 0)[2], None)
        assert_vector_equal(get_row(df, 1)[1], None)
        assert_vector_equal(get_row(df, 1)[2], FLOAT_VEC_3D_NP)
        assert_vector_equal(get_row(df, 2)[1], None)
        assert_vector_equal(get_row(df, 2)[2], None)


class TestFetchPandasVectorMultipleChunks:
    """Multiple-chunk download scenarios via fetch_pandas_batches."""

    def test_should_download_vector_data_in_multiple_chunks(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query generating 20000 integer vectors is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            "SELECT id, [id, id * 2, id * 3]::VECTOR(INT, 3) AS vec "
            "FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))) "
            "ORDER BY id",
        )

        # Then All 20000 rows should be fetched with valid 3-element integer vectors
        assert len(combined) == LARGE_RESULT_SET_SIZE
        vectors = get_column(combined, 1)
        for i, vec in enumerate(vectors):
            assert_vector_equal(vec, [i, i * 2, i * 3])
