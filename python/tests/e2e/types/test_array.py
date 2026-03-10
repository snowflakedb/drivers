"""ARRAY type tests for Universal Driver.

Snowflake ARRAY is a semi-structured data type storing ordered lists of values.
Values can be any Snowflake type including nested ARRAYs and OBJECTs.
Constructed via ARRAY_CONSTRUCT(val1, val2, ...).
Returned as JSON string or list depending on driver configuration.
Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured
"""

from __future__ import annotations

import json

from ...conftest import with_paramstyle


# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 10_000


def parse_array(value):
    """Parse an ARRAY value to list regardless of whether it's str or list."""
    if value is None:
        return None
    if isinstance(value, list):
        return value
    return json.loads(value)


class TestArrayTypeCasting:
    """Tests for ARRAY type casting to appropriate type."""

    def test_should_cast_array_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT ARRAY_CONSTRUCT(1, 2, 3)::ARRAY" is executed
        result = execute_query(
            "SELECT ARRAY_CONSTRUCT(1, 2, 3)::ARRAY",
            single_row=True,
        )

        # Then Value should be returned as appropriate type
        assert result[0] is not None
        parsed = parse_array(result[0])
        assert isinstance(parsed, list)

        # And Value should be an array containing elements [1, 2, 3]
        assert parsed == [1, 2, 3]


class TestArrayLiteral:
    """Tests for ARRAY type using SELECT with literals (no tables)."""

    def test_should_select_hardcoded_array_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT ARRAY_CONSTRUCT('a', 'b', 'c')" is executed
        result = execute_query(
            "SELECT ARRAY_CONSTRUCT('a', 'b', 'c')",
            single_row=True,
        )

        # Then Result should contain an array with 3 elements
        parsed = parse_array(result[0])
        assert isinstance(parsed, list)
        assert len(parsed) == 3

        # And Array values should be ['a', 'b', 'c']
        assert parsed == ["a", "b", "c"]

    def test_should_select_array_corner_case_values_from_literals(
        self, execute_query
    ):
        # Given Snowflake client is logged in

        # When Queries selecting corner case array literals are executed
        # Then Results should contain expected corner case array values

        result = execute_query(
            "SELECT ARRAY_CONSTRUCT()", single_row=True
        )
        parsed = parse_array(result[0])
        assert parsed == []

        result = execute_query(
            "SELECT ARRAY_CONSTRUCT(42)", single_row=True
        )
        parsed = parse_array(result[0])
        assert parsed == [42]

        result = execute_query(
            "SELECT ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1, 2), ARRAY_CONSTRUCT(3, 4))",
            single_row=True,
        )
        parsed = parse_array(result[0])
        assert len(parsed) == 2
        inner0 = parse_array(parsed[0]) if isinstance(parsed[0], str) else parsed[0]
        inner1 = parse_array(parsed[1]) if isinstance(parsed[1], str) else parsed[1]
        assert inner0 == [1, 2]
        assert inner1 == [3, 4]

        result = execute_query(
            "SELECT ARRAY_CONSTRUCT(1, 'two', TRUE)",
            single_row=True,
        )
        parsed = parse_array(result[0])
        assert len(parsed) == 3
        assert parsed[0] == 1
        assert parsed[1] == "two"
        assert parsed[2] is True

        result = execute_query(
            "SELECT ARRAY_CONSTRUCT(OBJECT_CONSTRUCT('key', 'value'))",
            single_row=True,
        )
        parsed = parse_array(result[0])
        assert len(parsed) == 1

        result = execute_query("SELECT NULL::ARRAY", single_row=True)
        assert result[0] is None


class TestArrayTable:
    """Tests for ARRAY type using table operations."""

    def test_should_select_array_values_from_table(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in

        # And A temporary table with VARIANT column is created
        table_name = f"{tmp_schema}.array_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # And The table is populated with array values
        execute_query(
            f"INSERT INTO {table_name} SELECT PARSE_JSON(column1) "
            f"FROM VALUES ('[1, 2, 3]'), ('[4, 5, 6]')"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted array values
        assert len(rows) == 2
        parsed_rows = [parse_array(row[0]) for row in rows]
        assert [1, 2, 3] in parsed_rows
        assert [4, 5, 6] in parsed_rows

    def test_should_select_array_corner_case_values_from_table(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in

        # And A temporary table with VARIANT column is created
        table_name = f"{tmp_schema}.array_corner_case_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # And The table is populated with corner case array values
        execute_query(
            f"INSERT INTO {table_name} SELECT PARSE_JSON('[]')"
        )
        execute_query(
            f"INSERT INTO {table_name} SELECT PARSE_JSON('[[1,2],[3,4]]')"
        )
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT PARSE_JSON('[1, \"two\", true]')"
        )
        execute_query(f"INSERT INTO {table_name} VALUES (NULL)")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted corner case array values
        assert len(rows) == 4

        non_null_rows = [row for row in rows if row[0] is not None]
        null_rows = [row for row in rows if row[0] is None]
        assert len(non_null_rows) == 3
        assert len(null_rows) == 1

        parsed = [parse_array(row[0]) for row in non_null_rows]
        assert [] in parsed


@with_paramstyle("qmark")
class TestArrayBinding:
    """Tests for ARRAY type using parameter binding."""

    def test_should_select_array_using_parameter_binding(
        self, execute_query
    ):
        # Given Snowflake client is logged in

        # When Query "SELECT PARSE_JSON(?)" is executed with bound JSON array string
        json_str = '[1, 2, 3]'
        result = execute_query(
            "SELECT PARSE_JSON(?)", (json_str,), single_row=True
        )

        # Then Result should contain a valid array
        parsed = parse_array(result[0])
        assert parsed == [1, 2, 3]

        # When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
        result = execute_query(
            "SELECT PARSE_JSON(?)", (None,), single_row=True
        )

        # Then Result should contain [NULL]
        assert result[0] is None

    def test_should_insert_array_using_parameter_binding(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in

        # And A temporary table with VARIANT column is created
        table_name = f"{tmp_schema}.array_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # When JSON array string is inserted using parameter binding via PARSE_JSON
        json_str = '[10, 20, 30]'
        execute_query(
            f"INSERT INTO {table_name} SELECT PARSE_JSON(?)",
            (json_str,),
        )

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted array
        assert len(rows) == 1
        parsed = parse_array(rows[0][0])
        assert parsed == [10, 20, 30]


class TestArrayMultipleChunks:
    """Tests for ARRAY type with multiple chunks downloading."""

    def test_should_download_array_data_in_multiple_chunks(
        self, execute_query
    ):
        # Given Snowflake client is logged in

        # When Query selecting 10000 ARRAY_CONSTRUCT rows from GENERATOR is executed
        sql = (
            "SELECT ARRAY_CONSTRUCT(seq8(), seq8() * 2, seq8() * 3) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        )
        rows = execute_query(sql)

        # Then there are 10000 rows returned
        assert len(rows) == LARGE_RESULT_SET_SIZE

        # And All returned values should be valid array representations
        for row in rows:
            parsed = parse_array(row[0])
            assert isinstance(parsed, list)
            assert len(parsed) == 3
