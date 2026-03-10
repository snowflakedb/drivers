"""VARIANT type tests for Universal Driver.

Snowflake VARIANT is a semi-structured data type that stores any JSON value:
objects, arrays, strings, numbers, booleans, and null.
Values are returned as JSON-serialized strings.
Maximum size: 16 MB (compressed)
Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured
"""

from __future__ import annotations

import json

from ...conftest import with_paramstyle
from .utils import assert_type


# =============================================================================
# CORNER CASE VALUES
# =============================================================================
# (expected_python_value, sql_expression)
CORNER_CASE_VALUES = [
    ({}, "PARSE_JSON('{}')"),
    ([], "PARSE_JSON('[]')"),
    (None, "PARSE_JSON('null')"),
    ({"a": {"b": {"c": 1}}}, """PARSE_JSON('{"a":{"b":{"c":1}}}')"""),
    ([1, "two", True, None], """PARSE_JSON('[1,"two",true,null]')"""),
    (True, "PARSE_JSON('true')"),
    (False, "PARSE_JSON('false')"),
    (42, "PARSE_JSON('42')"),
    (3.14, "PARSE_JSON('3.14')"),
    (-100, "PARSE_JSON('-100')"),
    ("hello", """PARSE_JSON('"hello"')"""),
]

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 1_000_000


def _parse_variant(value):
    """Parse a VARIANT result value (JSON string) into a Python object."""
    if value is None:
        return None
    return json.loads(value)


class TestVariantTypeCasting:
    """Tests for VARIANT type casting to appropriate type."""

    def test_should_cast_variant_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in

        # When Query selecting a JSON object as VARIANT is executed
        result = execute_query(
            """SELECT PARSE_JSON('{"key":"value"}')::VARIANT""",
            single_row=True,
        )

        # Then All values should be returned as string type
        assert_type(result, str)

        # And Value should be a valid JSON object
        parsed = _parse_variant(result[0])
        assert parsed == {"key": "value"}


class TestVariantLiteral:
    """Tests for VARIANT type using SELECT with literals (no tables)."""

    def test_should_select_variant_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query selecting JSON object and array as VARIANT literals is executed
        result = execute_query(
            """SELECT PARSE_JSON('{"name":"test","count":42}')::VARIANT, PARSE_JSON('[1,2,3]')::VARIANT""",
            single_row=True,
        )

        # Then Result should contain valid JSON object and array values
        assert_type(result, str)
        assert _parse_variant(result[0]) == {"name": "test", "count": 42}
        assert _parse_variant(result[1]) == [1, 2, 3]

    def test_should_select_variant_corner_case_values(self, execute_query):
        # Given Snowflake client is logged in

        # When Query selecting corner case VARIANT literals is executed
        # Then Result should contain expected corner case VARIANT values
        for expected_val, sql_expr in CORNER_CASE_VALUES:
            result = execute_query(f"SELECT {sql_expr}::VARIANT", single_row=True)
            assert _parse_variant(result[0]) == expected_val, (
                f"Expected {expected_val!r}, got {_parse_variant(result[0])!r}"
            )

    def test_should_handle_null_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query selecting VARIANT values with SQL NULL is executed
        result = execute_query(
            """SELECT PARSE_JSON('{"a":1}')::VARIANT, NULL::VARIANT, PARSE_JSON('[1]')::VARIANT""",
            single_row=True,
        )

        # Then Result should contain JSON values and SQL NULLs in expected positions
        assert _parse_variant(result[0]) == {"a": 1}
        assert result[1] is None
        assert _parse_variant(result[2]) == [1]


class TestVariantTable:
    """Tests for VARIANT type using table operations."""

    def test_should_select_variant_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with VARIANT column exists
        table_name = f"{tmp_schema}.variant_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # And VARIANT rows are inserted with PARSE_JSON values
        test_json_strings = [
            '{"name":"test","count":42}',
            '[1,2,3]',
            '"hello"',
        ]
        for json_str in test_json_strings:
            execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('{json_str}')")

        # When Query selecting all rows from VARIANT table is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain the inserted VARIANT values as JSON strings
        result = [_parse_variant(row[0]) for row in rows]
        expected = [json.loads(s) for s in test_json_strings]
        assert len(result) == len(expected)
        for val in expected:
            assert val in result, f"Expected {val!r} in result"

    def test_should_handle_null_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with VARIANT column exists
        table_name = f"{tmp_schema}.variant_null_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # And VARIANT rows including NULLs are inserted
        execute_query(f"INSERT INTO {table_name} VALUES (NULL)")
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('{{\"a\":1}}')")
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('[1,2]')")

        # When Query selecting all rows from VARIANT table is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain NULL and non-NULL VARIANT values in any order
        result = [_parse_variant(row[0]) for row in rows]
        assert None in result
        assert dict(a=1) in result
        assert [1, 2] in result


@with_paramstyle("qmark")
class TestVariantBinding:
    """Tests for VARIANT type using parameter binding."""

    def test_should_select_variant_using_parameter_binding_with_parse_json(self, execute_query):
        # Given Snowflake client is logged in

        # When Query with PARSE_JSON binding is executed with a JSON string parameter
        result = execute_query(
            "SELECT PARSE_JSON(?)::VARIANT",
            ('{"key":"value"}',),
            single_row=True,
        )

        # Then Result should contain the expected JSON value
        assert_type(result, str)
        expected = dict(key="value")
        assert _parse_variant(result[0]) == expected

        # When Query with PARSE_JSON binding is executed with NULL parameter
        result = execute_query("SELECT PARSE_JSON(?)::VARIANT", (None,), single_row=True)

        # Then Result should contain NULL
        assert result == (None,)

    def test_should_insert_variant_using_parameter_binding(self, execute_query, executemany_insert, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with VARIANT column exists
        table_name = f"{tmp_schema}.variant_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")

        # When VARIANT values are inserted using parameter binding with PARSE_JSON
        test_json_strings = [
            ('{"key":"value"}',),
            ('[1,2,3]',),
            (None,),
        ]
        rows = executemany_insert(
            table_name,
            f"INSERT INTO {table_name} SELECT PARSE_JSON(?)",
            test_json_strings,
        )

        # Then SELECT should return the same VARIANT values
        result = [_parse_variant(row[0]) for row in rows]
        assert {"key": "value"} in result
        assert [1, 2, 3] in result
        assert None in result


class TestVariantMultipleChunks:
    """Tests for VARIANT type with multiple chunks downloading."""

    def test_should_download_large_result_set_with_multiple_chunks_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with VARIANT column exists with 1000000 generated VARIANT values
        table_name = f"{tmp_schema}.large_variant_table"
        execute_query(f"CREATE TABLE {table_name} (col VARIANT)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT PARSE_JSON('{{\"id\":' || seq8() || '}}') "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query selecting all rows from VARIANT table is executed
        rows = execute_query(f"SELECT col FROM {table_name}")

        # Then Result should contain 1000000 VARIANT values
        assert len(rows) == LARGE_RESULT_SET_SIZE
        assert_type([row[0] for row in rows], str)
        sample = _parse_variant(rows[0][0])
        assert "id" in sample
