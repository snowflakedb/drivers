"""Semi-structured type (VARIANT/OBJECT/ARRAY) tests for Universal Driver.

This module tests semi-structured types (VARIANT, OBJECT, ARRAY) across various scenarios
including literals, table operations, NULL handling, empty containers, unicode content,
parameter binding, and large result sets.

Snowflake semi-structured types: VARIANT, OBJECT, ARRAY
Internal representation: Python connector returns these as JSON strings (str type).
Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured
"""

from __future__ import annotations

import json

from ...conftest import with_paramstyle
from .utils import assert_type


# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 20_000


def parse_json_value(value):
    """Parse a JSON string value returned by Snowflake, returning None for SQL NULLs."""
    if value is None:
        return None
    return json.loads(value)


class TestSemiStructuredTypeCasting:
    """Tests for semi-structured type casting to appropriate type."""

    def test_should_cast_semi_structured_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
        sql = "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, str)

        # And Parsed JSON values should match expected structures
        assert parse_json_value(result[0]) == {"a": 1}
        assert parse_json_value(result[1]) == [1, 2, 3]
        assert parse_json_value(result[2]) == {"key": "val"}


class TestSemiStructuredLiteral:
    """Tests for semi-structured types using SELECT with literals (no tables)."""

    def test_should_select_semi_structured_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30),
        # OBJECT_CONSTRUCT('a', 1, 'b', 2)" is executed
        sql = "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)"
        result = execute_query(sql, single_row=True)

        # Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns
        assert_type(result, str)
        assert parse_json_value(result[0]) == {"key": "value"}
        assert parse_json_value(result[1]) == [10, 20, 30]
        assert parse_json_value(result[2]) == {"a": 1, "b": 2}

    def test_should_select_deeply_nested_semi_structured_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
        sql = 'SELECT PARSE_JSON(\'{"a":{"b":[1,2,{"c":true}]}}\')'
        result = execute_query(sql, single_row=True)

        # Then Result should contain the expected nested value
        assert isinstance(result[0], str)
        parsed = parse_json_value(result[0])
        assert parsed == {"a": {"b": [1, 2, {"c": True}]}}

    def test_should_handle_null_semi_structured_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
        sql = "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY"
        result = execute_query(sql, single_row=True)

        # Then All columns should return null indicators
        assert result == (None, None, None)


class TestSemiStructuredTable:
    """Tests for semi-structured types using table operations."""

    def test_should_select_semi_structured_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT, OBJECT, and ARRAY columns exists with JSON values
        table_name = f"{tmp_schema}.semi_structured_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (var_col VARIANT, obj_col OBJECT, arr_col ARRAY)"
        )
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT PARSE_JSON('{{\"x\":42}}'), "
            f"OBJECT_CONSTRUCT('key', 'value'), "
            f"ARRAY_CONSTRUCT(1, 2, 3)"
        )

        # When Query "SELECT * FROM <table>" is executed
        result = execute_query(f"SELECT * FROM {table_name}", single_row=True)

        # Then Data should contain the expected semi-structured values
        assert_type(result, str)
        assert parse_json_value(result[0]) == {"x": 42}
        assert parse_json_value(result[1]) == {"key": "value"}
        assert parse_json_value(result[2]) == [1, 2, 3]

    def test_should_handle_null_semi_structured_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT column exists containing NULLs and values
        table_name = f"{tmp_schema}.semi_structured_null_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col VARIANT, id INT)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT PARSE_JSON(column2), column1 FROM VALUES (1, NULL), (2, '{{\"a\":1}}'), (3, NULL)"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT col FROM {table_name} ORDER BY id")

        # Then Result should contain [NULL, {"a":1}, NULL]
        values = [row[0] for row in rows]
        assert len(values) == 3
        parsed = [parse_json_value(v) for v in values]
        assert parsed == [None, {"a": 1}, None]


class TestSemiStructuredEmptyContainers:
    """Tests for empty JSON container handling."""

    def test_should_handle_empty_json_containers(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
        sql = "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()"
        result = execute_query(sql, single_row=True)

        # Then Each column should return a valid empty container
        assert parse_json_value(result[0]) == {}
        assert parse_json_value(result[1]) == []
        assert parse_json_value(result[2]) == {}

    def test_should_handle_empty_json_array_literal(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('[]')" is executed
        sql = "SELECT PARSE_JSON('[]')"
        result = execute_query(sql, single_row=True)

        # Then Result should be an empty JSON array
        assert parse_json_value(result[0]) == []

    def test_should_round_trip_empty_json_containers_through_a_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT, OBJECT, and ARRAY columns exists with empty containers
        table_name = f"{tmp_schema}.empty_container_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (var_col VARIANT, obj_col OBJECT, arr_col ARRAY)"
        )
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('{{}}'), OBJECT_CONSTRUCT(), ARRAY_CONSTRUCT()")

        # When Query "SELECT * FROM <table>" is executed
        result = execute_query(f"SELECT * FROM {table_name}", single_row=True)

        # Then All columns should return valid empty containers
        assert parse_json_value(result[0]) == {}
        assert parse_json_value(result[1]) == {}
        assert parse_json_value(result[2]) == []


class TestSemiStructuredUnicode:
    """Tests for JSON with unicode content."""

    def test_should_handle_json_with_unicode_content(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query returning JSON with unicode characters is executed
        sql = 'SELECT PARSE_JSON(\'{"greeting":"こんにちは","emoji":"⛄"}\')'
        result = execute_query(sql, single_row=True)

        # Then Result should preserve the unicode characters
        parsed = parse_json_value(result[0])
        assert parsed["greeting"] == "こんにちは"
        assert parsed["emoji"] == "⛄"

    def test_should_handle_json_with_unicode_in_keys(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query returning JSON with unicode characters in keys is executed
        sql = 'SELECT PARSE_JSON(\'{"名前":"テスト","données":"valeur"}\')'
        result = execute_query(sql, single_row=True)

        # Then Result should preserve unicode keys and their associated values
        parsed = parse_json_value(result[0])
        assert parsed["名前"] == "テスト"
        assert parsed["données"] == "valeur"


class TestSemiStructuredMultipleChunks:
    """Tests for semi-structured types with multiple chunks downloading."""

    def test_should_download_semi_structured_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v
        # ORDER BY 1" is executed
        sql = (
            f"SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v ORDER BY 1"
        )
        rows = execute_query(sql)

        # Then All 20000 rows should be fetched and each should contain a value with "id" key
        assert len(rows) == LARGE_RESULT_SET_SIZE
        for row in rows:
            parsed = parse_json_value(row[0])
            assert "id" in parsed


@with_paramstyle("qmark")
class TestSemiStructuredBinding:
    """Tests for semi-structured types using parameter binding."""

    def test_should_select_variant_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string '{"bound":true}'
        sql = "SELECT PARSE_JSON(?)"
        result = execute_query(sql, ('{"bound":true}',), single_row=True)

        # Then Result should contain a value with "bound" key
        parsed = parse_json_value(result[0])
        assert parsed["bound"] is True

    def test_should_select_null_variant_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
        sql = "SELECT PARSE_JSON(?)"
        result = execute_query(sql, (None,), single_row=True)

        # Then Result should be NULL
        assert result == (None,)

    def test_should_insert_variant_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT column exists
        table_name = f"{tmp_schema}.variant_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col VARIANT, id INT)")

        # When JSON values are inserted using parameter binding via PARSE_JSON(?)
        test_values = [('{"x":1}',), ("[1,2,3]",), ('{"nested":{"a":true}}',)]
        for i, params in enumerate(test_values, 1):
            # Uses a loop instead of executemany because INSERT ... SELECT PARSE_JSON(?)
            # is incompatible with Snowflake's server-side array binding (VALUES-only).
            execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON(?), {i}", params)

        # Then SELECT should return the inserted JSON values
        rows = execute_query(f"SELECT col FROM {table_name} ORDER BY id")
        assert len(rows) == 3
        assert parse_json_value(rows[0][0]) == {"x": 1}
        assert parse_json_value(rows[1][0]) == [1, 2, 3]
        assert parse_json_value(rows[2][0]) == {"nested": {"a": True}}
