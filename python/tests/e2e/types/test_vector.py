"""VECTOR type tests for Universal Driver.

This module tests VECTOR type which stores fixed-size arrays of numeric values.
Subtypes: INT (integer) and FLOAT (32-bit floating-point).
Values are returned as Python lists (list[int] or list[float]).

Reference: https://docs.snowflake.com/en/sql-reference/data-types-vector
"""

from __future__ import annotations

import pytest

from .utils import assert_type


# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 20_000


class TestVectorTypeCasting:
    """Tests for VECTOR type casting to appropriate type."""

    def test_should_cast_vector_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
        sql = "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, list)
        assert result[0] == [1, 2, 3]
        assert result[1] == pytest.approx([1.5, 2.5, 3.5])


class TestVectorLiteral:
    """Tests for VECTOR type using SELECT with literals (no tables)."""

    def test_should_select_integer_vector_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 3, -5]::VECTOR(INT, 3), [40, 1234567]::VECTOR(INT, 2)" is executed
        sql = "SELECT [1, 3, -5]::VECTOR(INT, 3), [40, 1234567]::VECTOR(INT, 2)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain integer vectors [1, 3, -5] and [40, 1234567]
        assert isinstance(result[0], list)
        assert isinstance(result[1], list)
        assert result[0] == [1, 3, -5]
        assert result[1] == [40, 1234567]

    def test_should_select_float_vector_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1.8, -3.4, 6.7, 0, 2.3]::VECTOR(FLOAT, 5)" is executed
        sql = "SELECT [1.8, -3.4, 6.7, 0, 2.3]::VECTOR(FLOAT, 5)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain float vector [1.8, -3.4, 6.7, 0, 2.3]
        assert isinstance(result[0], list)
        assert result[0] == pytest.approx([1.8, -3.4, 6.7, 0.0, 2.3])

    def test_should_handle_null_vector_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
        sql = "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [[1, 2, 3], NULL, NULL]
        assert result[0] == [1, 2, 3]
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
            f"SELECT 1, [1, 2, 3]::VECTOR(INT, 3), [1.1, 2.2, 3.3, 4.4, 5.5]::VECTOR(FLOAT, 5) "
            f"UNION ALL SELECT 2, [10, 20, 30]::VECTOR(INT, 3), [10.5, 20.5, 30.5, 40.5, 50.5]::VECTOR(FLOAT, 5)"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT int_vec, float_vec FROM {table_name} ORDER BY id")

        # Then Result should contain the expected integer and float vector values
        assert len(rows) == 2
        assert rows[0][0] == [1, 2, 3]
        assert rows[0][1] == pytest.approx([1.1, 2.2, 3.3, 4.4, 5.5])
        assert rows[1][0] == [10, 20, 30]
        assert rows[1][1] == pytest.approx([10.5, 20.5, 30.5, 40.5, 50.5])

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
            f"SELECT 1, [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3) "
            f"UNION ALL SELECT 2, NULL::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3) "
            f"UNION ALL SELECT 3, NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)"
        )

        # When Query "SELECT * FROM <table> ORDER BY id" is executed
        rows = execute_query(f"SELECT int_vec, float_vec FROM {table_name} ORDER BY id")

        # Then Result should contain both vector values and NULLs
        assert len(rows) == 3
        assert rows[0][0] == [1, 2, 3]
        assert rows[0][1] is None
        assert rows[1][0] is None
        assert rows[1][1] == pytest.approx([1.5, 2.5, 3.5])
        assert rows[2][0] is None
        assert rows[2][1] is None


class TestVectorMultipleChunks:
    """Tests for VECTOR type with multiple chunks downloading."""

    def test_should_download_vector_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT [seq8(), seq8() * 2, seq8() * 3]::VECTOR(INT, 3) AS vec
        # FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
        sql = (
            f"SELECT [seq8(), seq8() * 2, seq8() * 3]::VECTOR(INT, 3) AS vec "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        )
        rows = execute_query(sql)

        # Then All 20000 rows should be fetched and each should be a non-null list value
        assert len(rows) == LARGE_RESULT_SET_SIZE
        for row in rows:
            assert isinstance(row[0], list)


class TestVectorJsonResultFormat:
    """Tests for VECTOR type with JSON result format."""

    def test_should_select_vector_with_json_result_format(self, connection_factory):
        # Given Snowflake client is logged in
        pass

        # And Session parameter PYTHON_CONNECTOR_QUERY_RESULT_FORMAT is set to JSON
        with connection_factory(session_parameters={"PYTHON_CONNECTOR_QUERY_RESULT_FORMAT": "JSON"}) as conn:
            with conn.cursor() as cursor:
                # When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
                sql = "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)"
                cursor.execute(sql)
                result = cursor.fetchone()

                # Then Result should contain the expected integer and float vector values
                assert isinstance(result[0], list)
                assert isinstance(result[1], list)
                assert result[0] == [1, 2, 3]
                assert result[1] == pytest.approx([1.5, 2.5, 3.5])
