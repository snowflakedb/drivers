"""OBJECT type tests for Universal Driver.

Snowflake OBJECT is a semi-structured data type storing key-value pairs.
Keys are always strings; values can be any Snowflake type.
Constructed via OBJECT_CONSTRUCT('key1', val1, 'key2', val2, ...).
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


def parse_object(value):
    """Parse an OBJECT value to dict regardless of whether it's str or dict."""
    if value is None:
        return None
    if isinstance(value, dict):
        return value
    return json.loads(value)


class TestObjectTypeCasting:
    """Tests for OBJECT type casting to appropriate type."""

    def test_should_cast_object_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT OBJECT_CONSTRUCT('name', 'Alice', 'age', 30)::OBJECT" is executed
        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('name', 'Alice', 'age', 30)::OBJECT",
            single_row=True,
        )

        # Then Value should be returned as appropriate type
        assert result[0] is not None
        parsed = parse_object(result[0])
        assert isinstance(parsed, dict)

        # And Value should contain key 'name' with value 'Alice' and key 'age' with value 30
        assert parsed["name"] == "Alice"
        assert parsed["age"] == 30


class TestObjectLiteral:
    """Tests for OBJECT type using SELECT with literals (no tables)."""

    def test_should_select_hardcoded_object_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT OBJECT_CONSTRUCT('key1', 'value1', 'key2', 42)" is executed
        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('key1', 'value1', 'key2', 42)",
            single_row=True,
        )

        # Then Result should contain an object with keys [key1, key2]
        parsed = parse_object(result[0])
        assert isinstance(parsed, dict)
        assert set(parsed.keys()) == set(["key1", "key2"])

        # And Object values should be key1='value1' and key2=42
        assert parsed["key1"] == "value1"
        assert parsed["key2"] == 42

    def test_should_select_object_corner_case_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Queries selecting corner case object literals are executed
        # Then Results should contain expected corner case object values

        result = execute_query("SELECT OBJECT_CONSTRUCT()", single_row=True)
        parsed = parse_object(result[0])
        assert parsed == dict()

        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('key', NULL)",
            single_row=True,
        )
        parsed = parse_object(result[0])
        assert "key" not in parsed, "Snowflake omits NULL-valued keys by default"

        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('outer', OBJECT_CONSTRUCT('inner', 'value'))",
            single_row=True,
        )
        parsed = parse_object(result[0])
        assert "outer" in parsed
        inner = parse_object(parsed["outer"]) if isinstance(parsed["outer"], str) else parsed["outer"]
        assert inner["inner"] == "value"

        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('flag', TRUE)",
            single_row=True,
        )
        parsed = parse_object(result[0])
        assert parsed["flag"] is True

        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('int', 1, 'float', 1.5)",
            single_row=True,
        )
        parsed = parse_object(result[0])
        assert parsed["int"] == 1
        assert parsed["float"] == 1.5

        result = execute_query(
            "SELECT OBJECT_CONSTRUCT('日本語', 'テスト')",
            single_row=True,
        )
        parsed = parse_object(result[0])
        assert parsed["日本語"] == "テスト"

        result = execute_query("SELECT NULL::OBJECT", single_row=True)
        assert result[0] is None


class TestObjectTable:
    """Tests for OBJECT type using table operations."""

    def test_should_select_object_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And A temporary table with VARIANT column is created
        table_name = f"{tmp_schema}.object_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # And The table is populated with object values
        alice_json = '{"name": "Alice", "age": 30}'
        bob_json = '{"name": "Bob", "age": 25}'
        execute_query(
            f"INSERT INTO {table_name} SELECT PARSE_JSON(column1) FROM VALUES ('{alice_json}'), ('{bob_json}')"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted object values
        assert len(rows) == 2
        parsed_rows = [parse_object(row[0]) for row in rows]
        names = set(r["name"] for r in parsed_rows)
        assert names == set(["Alice", "Bob"])

    def test_should_select_object_corner_case_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And A temporary table with VARIANT column is created
        table_name = f"{tmp_schema}.object_corner_case_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # And The table is populated with corner case object values
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('{{}}')")
        execute_query(f'INSERT INTO {table_name} SELECT PARSE_JSON(\'{{"nested": {{"key": "value"}}}}\')')
        mixed_json = '{"str": "hello", "num": 42, "bool": true, "arr": [1,2,3]}'
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('{mixed_json}')")
        execute_query(f"INSERT INTO {table_name} VALUES (NULL)")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted corner case object values
        assert len(rows) == 4

        non_null_rows = [row for row in rows if row[0] is not None]
        null_rows = [row for row in rows if row[0] is None]
        assert len(non_null_rows) == 3
        assert len(null_rows) == 1

        parsed = [parse_object(row[0]) for row in non_null_rows]
        assert dict() in parsed
        nested = next(p for p in parsed if "nested" in p)
        inner = parse_object(nested["nested"]) if isinstance(nested["nested"], str) else nested["nested"]
        assert inner["key"] == "value"


@with_paramstyle("qmark")
class TestObjectBinding:
    """Tests for OBJECT type using parameter binding."""

    def test_should_select_object_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string
        json_str = '{"key": "value"}'
        result = execute_query("SELECT PARSE_JSON(?)", (json_str,), single_row=True)

        # Then Result should contain a valid object
        parsed = parse_object(result[0])
        assert parsed == dict(key="value")

        # When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
        result = execute_query("SELECT PARSE_JSON(?)", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result[0] is None

    def test_should_insert_object_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And A temporary table with VARIANT column is created
        table_name = f"{tmp_schema}.object_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # When JSON string is inserted using parameter binding via PARSE_JSON
        json_str = '{"name": "test", "value": 42}'
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON(?)", (json_str,))

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted object
        assert len(rows) == 1
        parsed = parse_object(rows[0][0])
        assert parsed["name"] == "test"
        assert parsed["value"] == 42


class TestObjectMultipleChunks:
    """Tests for OBJECT type with multiple chunks downloading."""

    def test_should_download_object_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in

        # When Query selecting 10000 OBJECT_CONSTRUCT rows from GENERATOR is executed
        sql = (
            "SELECT OBJECT_CONSTRUCT('id', seq8(), 'value', TO_VARCHAR(seq8())) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v"
        )
        rows = execute_query(sql)

        # Then there are 10000 rows returned
        assert len(rows) == LARGE_RESULT_SET_SIZE

        # And All returned values should be valid object representations
        for row in rows:
            parsed = parse_object(row[0])
            assert isinstance(parsed, dict)
            assert "id" in parsed
            assert "value" in parsed
