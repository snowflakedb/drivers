"""MAP type tests for Universal Driver.

Snowflake MAP is a semi-structured data type storing key-value pairs
with typed keys and typed values. Unlike OBJECT, MAP keys are not
restricted to strings.
Created by casting OBJECT to MAP or using MAP-typed columns.
Returned as JSON string or dict depending on driver configuration.
Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured
"""

from __future__ import annotations

import json

from ...conftest import with_paramstyle


# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 10_000

MAP_SQL = (
    "SELECT OBJECT_CONSTRUCT('x', '1', 'y', '2')"
    "::MAP(VARCHAR, VARCHAR)"
)

MAP_INT_SQL = (
    "SELECT OBJECT_CONSTRUCT('a', 1, 'b', 2)"
    "::MAP(VARCHAR, INTEGER)"
)


def parse_map(value):
    """Parse a MAP value to dict regardless of whether it's str or dict."""
    if value is None:
        return None
    if isinstance(value, dict):
        return value
    return json.loads(value)


class TestMapTypeCasting:
    """Tests for MAP type casting to appropriate type."""

    def test_should_cast_map_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in

        # When Query selecting a MAP(VARCHAR, VARCHAR) value is executed
        result = execute_query(MAP_SQL, single_row=True)

        # Then Value should be returned as appropriate type
        assert result[0] is not None
        parsed = parse_map(result[0])
        assert isinstance(parsed, dict)

        # And Value should be a map containing key 'x' with value '1' and key 'y' with value '2'
        assert str(parsed["x"]) == "1"
        assert str(parsed["y"]) == "2"


class TestMapLiteral:
    """Tests for MAP type using SELECT with literals (no tables)."""

    def test_should_select_hardcoded_map_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query selecting a MAP(VARCHAR, INTEGER) value with keys [a, b] is executed
        result = execute_query(MAP_INT_SQL, single_row=True)

        # Then Result should contain a map with 2 entries
        parsed = parse_map(result[0])
        assert isinstance(parsed, dict)
        assert len(parsed) == 2

        # And Map values should be a=1 and b=2
        assert parsed["a"] == 1
        assert parsed["b"] == 2

    def test_should_select_map_corner_case_values_from_literals(
        self, execute_query
    ):
        # Given Snowflake client is logged in

        # When Queries selecting corner case map literals are executed
        # Then Results should contain expected corner case map values

        result = execute_query(
            "SELECT OBJECT_CONSTRUCT()::MAP(VARCHAR, VARCHAR)",
            single_row=True,
        )
        parsed = parse_map(result[0])
        assert parsed == {}

        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('only', 'one')::MAP(VARCHAR, VARCHAR)",
            single_row=True,
        )
        parsed = parse_map(result[0])
        assert len(parsed) == 1
        assert parsed["only"] == "one"

        result = execute_query(
            "SELECT NULL::MAP(VARCHAR, VARCHAR)", single_row=True
        )
        assert result[0] is None


class TestMapTable:
    """Tests for MAP type using table operations."""

    def test_should_select_map_values_from_table(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in

        # And A temporary table with MAP column is created
        table_name = f"{tmp_schema}.map_table"
        execute_query(
            f"CREATE TABLE {table_name} (col MAP(VARCHAR, VARCHAR))"
        )

        # And The table is populated with map values
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT OBJECT_CONSTRUCT('k1', 'v1')::MAP(VARCHAR, VARCHAR)"
        )
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT OBJECT_CONSTRUCT('k2', 'v2')::MAP(VARCHAR, VARCHAR)"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted map values
        assert len(rows) == 2
        parsed_rows = [parse_map(row[0]) for row in rows]
        keys = set()
        for p in parsed_rows:
            keys.update(p.keys())
        assert "k1" in keys
        assert "k2" in keys

    def test_should_select_map_corner_case_values_from_table(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in

        # And A temporary table with MAP column is created
        table_name = f"{tmp_schema}.map_corner_case_table"
        execute_query(
            f"CREATE TABLE {table_name} (col MAP(VARCHAR, VARCHAR))"
        )

        # And The table is populated with corner case map values
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT OBJECT_CONSTRUCT()::MAP(VARCHAR, VARCHAR)"
        )
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT OBJECT_CONSTRUCT('a', 'b')::MAP(VARCHAR, VARCHAR)"
        )
        execute_query(
            f"INSERT INTO {table_name} SELECT NULL::MAP(VARCHAR, VARCHAR)"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted corner case map values
        assert len(rows) == 3

        non_null_rows = [row for row in rows if row[0] is not None]
        null_rows = [row for row in rows if row[0] is None]
        assert len(non_null_rows) == 2
        assert len(null_rows) == 1


@with_paramstyle("qmark")
class TestMapBinding:
    """Tests for MAP type using parameter binding."""

    def test_should_select_map_using_parameter_binding(
        self, execute_query
    ):
        # Given Snowflake client is logged in

        # When Query selecting PARSE_JSON with bound JSON map string is executed
        json_str = '{"key": "value"}'
        result = execute_query(
            "SELECT PARSE_JSON(?)", (json_str,), single_row=True
        )

        # Then Result should contain a valid map
        parsed = parse_map(result[0])
        assert parsed == dict(key="value")

        # When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
        result = execute_query(
            "SELECT PARSE_JSON(?)", (None,), single_row=True
        )

        # Then Result should contain [NULL]
        assert result[0] is None

    def test_should_insert_map_using_parameter_binding(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in

        # And A temporary table with MAP column is created
        table_name = f"{tmp_schema}.map_bind_table"
        execute_query(
            f"CREATE TABLE {table_name} (col MAP(VARCHAR, VARCHAR))"
        )

        # When JSON map string is inserted using parameter binding
        json_str = '{"name": "test"}'
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT PARSE_JSON(?)::MAP(VARCHAR, VARCHAR)",
            (json_str,),
        )

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted map
        assert len(rows) == 1
        parsed = parse_map(rows[0][0])
        assert parsed["name"] == "test"


class TestMapMultipleChunks:
    """Tests for MAP type with multiple chunks downloading."""

    def test_should_download_map_data_in_multiple_chunks(
        self, execute_query
    ):
        # Given Snowflake client is logged in

        # When Query selecting 10000 MAP rows from GENERATOR is executed
        sql = (
            "SELECT OBJECT_CONSTRUCT("
            "'id', TO_VARCHAR(seq8()), "
            "'val', TO_VARCHAR(seq8() * 10)"
            ")::MAP(VARCHAR, VARCHAR) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        )
        rows = execute_query(sql)

        # Then there are 10000 rows returned
        assert len(rows) == LARGE_RESULT_SET_SIZE

        # And All returned values should be valid map representations
        for row in rows:
            parsed = parse_map(row[0])
            assert isinstance(parsed, dict)
            assert "id" in parsed
            assert "val" in parsed
