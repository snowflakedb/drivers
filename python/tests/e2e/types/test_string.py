"""STRING type tests for Universal Driver.

This module tests STRING type and its synonyms (VARCHAR, CHAR, CHARACTER, NCHAR, STRING, TEXT,
VARCHAR2, NVARCHAR, NVARCHAR2, CHAR VARYING, NCHAR VARYING) across various scenarios including
literals, table operations, corner cases, NULL handling, parameter binding, and large result sets.

All tests are parameterized to run with each type synonym to verify they behave identically.

Snowflake String types: All are synonymous with VARCHAR and store Unicode UTF-8.
Maximum length: 134,217,728 characters (default 16,777,216 if unspecified)
Reference: https://docs.snowflake.com/en/sql-reference/data-types-text
"""

import pytest

from ...conftest import with_paramstyle
from .utils import assert_sequential_values, assert_type


# =============================================================================
# TYPE SYNONYMS
# =============================================================================
# https://docs.snowflake.com/en/sql-reference/data-types-text
STRING_TYPE_SYNONYMS = [
    "VARCHAR",
    "CHAR",
    "CHARACTER",
    "NCHAR",
    "STRING",
    "TEXT",
    "VARCHAR2",
    "NVARCHAR",
    "NVARCHAR2",
    "CHAR VARYING",
    "NCHAR VARYING",
]
string_type_parametrize = pytest.mark.parametrize("string_type", STRING_TYPE_SYNONYMS)

# =============================================================================
# CORNER CASE VALUES
# =============================================================================
# Corner cases for string testing:
#   - Empty string: ''
#   - Single character: 'X'
#   - Whitespace only: '   '
#   - Tab character: '\t'
#   - Newline: '\n'
#   - Unicode snowman: '\u26c4' (⛄)
#   - Unicode characters: '日本語テスト' (Japanese)
#   - Escaped single quote: '\''
#   - Escaped backslash: '\\'
#   - NULL value
#   - Combined character: 'y̆es' (character with combining diacritical mark)
#   - Surrogate pair: '\U0001D11E' (𝄞 musical G clef)


# SQL representations for corner case strings (for INSERT statements)
CORNER_CASE_VALUES = [
    ("", "''"),  # Empty string
    ("X", "'X'"),  # Single character
    ("   ", "'   '"),  # Whitespace only
    ("\t", "'\\t'"),  # Tab character
    ("\n", "'\\n'"),  # Newline
    ("⛄", "'⛄'"),  # Unicode snowman
    ("日本語テスト", "'日本語テスト'"),  # Japanese
    ("'", "''''"),  # Single quote
    ("\\", "'\\\\'"),  # Backslash
    (None, "NULL"),  # NULL value
    ("y̆es", "'y̆es'"),  # Combined character
    ("𝄞", "'𝄞'"),  # Musical G clef
]

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 10_000


class TestStringTypeCasting:
    """Tests for STRING type casting to appropriate type."""

    @string_type_parametrize
    def test_should_cast_string_values_to_appropriate_type_for_string_and_synonyms(self, execute_query, string_type):
        # Given Snowflake client is logged in

        # When Query "SELECT 'hello'::<type>, 'Hello World'::<type>, '日本語テスト'::<type>" is executed
        sql = f"SELECT 'hello'::{string_type}(32), 'Hello World'::{string_type}(32), '日本語テスト'::{string_type}(32)"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, str)
        assert result == ("hello", "Hello World", "日本語テスト")


class TestStringLiteral:
    """Tests for STRING type using SELECT with literals (no tables)."""

    @string_type_parametrize
    def test_should_select_hardcoded_string_literals(self, execute_query, string_type):
        # Given Snowflake client is logged in

        # When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
        sql = (
            f"SELECT 'hello'::{string_type}(32) AS str1, "
            f"'Hello World'::{string_type}(32) AS str2, "
            f"'Snowflake Driver Test'::{string_type}(32) AS str3"
        )
        result = execute_query(sql, single_row=True)

        # Then the result should contain:
        assert_type(result, str)
        assert result == ("hello", "Hello World", "Snowflake Driver Test")

    @string_type_parametrize
    def test_should_select_string_literals_with_corner_case_values(self, execute_query, string_type):
        # Corner cases: empty string, single character, whitespace-only, unicode characters, escape sequences

        # Given Snowflake client is logged in

        # When Query selecting corner case string literals is executed

        # Then the result should contain expected corner case string values
        for expected_val, sql_val in CORNER_CASE_VALUES:
            result = execute_query(f"SELECT {sql_val}::{string_type}(32)", single_row=True)
            assert result == (expected_val,), f"Expected {expected_val!r}, got {result[0]!r}"


class TestStringTable:
    """Tests for STRING type using table operations."""

    @string_type_parametrize
    def test_should_select_hardcoded_string_values_from_table(self, execute_query, tmp_schema, string_type):
        # Given Snowflake client is logged in

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.string_table_test"
        execute_query(f"CREATE TABLE {table_name} (val {string_type}(32))")

        # And The table is populated with string values
        test_values = ["hello", "Hello World", "Snowflake Driver Test"]
        for val in test_values:
            execute_query(f"INSERT INTO {table_name} VALUES ('{val}')")

        # When Query "SELECT * FROM {table}" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then the result should contain the inserted hardcoded string values
        result = set(row[0] for row in rows)
        assert_type(result, str)
        assert result == set(test_values)

    @string_type_parametrize
    def test_should_select_corner_case_string_values_from_table(self, execute_query, tmp_schema, string_type):
        # Given Snowflake client is logged in

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.string_corner_case_table_test"
        execute_query(f"CREATE TABLE {table_name} (val {string_type}(32))")

        # And The table is populated with corner case string values
        for _, sql_val in CORNER_CASE_VALUES:
            execute_query(f"INSERT INTO {table_name} VALUES ({sql_val})")

        # When Query "SELECT * FROM {table}" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then the result should contain the inserted corner case string values
        result = [row[0] for row in rows]
        expected = [expected_val for expected_val, _ in CORNER_CASE_VALUES]
        assert len(result) == len(expected)
        assert_type(result, str, can_be_none=True)
        assert set(result) == set(expected)


@with_paramstyle("qmark")
class TestStringBinding:
    """Tests for STRING type using parameter binding."""

    @string_type_parametrize
    def test_should_insert_and_select_back_hardcoded_string_values_using_parameter_binding(
        self, execute_query, executemany_insert, tmp_schema, string_type
    ):
        # Given Snowflake client is logged in

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.string_bind_table_test"
        execute_query(f"CREATE TABLE {table_name} (val {string_type}(32))")

        # When String value 'Test binding value 日本語' is inserted using parameter binding
        test_value = "Test binding value 日本語"
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", [(test_value,)])

        # And Query "SELECT * FROM {table}" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then the result should contain the bound string value 'Test binding value 日本語'
        result = [row[0] for row in rows]
        assert len(result) == 1
        assert_type(result[0], str)
        assert result == [test_value]

    @string_type_parametrize
    def test_should_select_string_literals_using_parameter_binding(self, execute_query, string_type):
        # SELECT binding test: Uses SELECT ?::VARCHAR to bind string values

        # Given Snowflake client is logged in

        # When Query "SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR" is executed
        # with bound string values ['hello', 'Hello World', '日本語テスト']
        result = execute_query(
            f"SELECT ?::{string_type}(32), ?::{string_type}(32), ?::{string_type}(32)",
            ("hello", "Hello World", "日本語テスト"),
            single_row=True,
        )

        # Then the result should contain:
        assert_type(result, str)
        assert result == ("hello", "Hello World", "日本語テスト")

    @string_type_parametrize
    def test_should_select_corner_case_string_values_using_parameter_binding(self, execute_query, string_type):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::VARCHAR" is executed with each corner case string value bound
        for corner_case, _ in CORNER_CASE_VALUES:
            result = execute_query(f"SELECT ?::{string_type}(32)", (corner_case,), single_row=True)

            # Then the result should match the bound corner case value
            assert result == (corner_case,)


class TestStringMultipleChunks:
    """Tests for STRING type with multiple chunks downloading."""

    def test_should_download_string_data_in_multiple_chunks(self, execute_query):
        # This test ensures proper handling of large result sets that span multiple chunks
        # ~10000 values ensures data is downloaded in at least two chunks

        # Given Snowflake client is logged in

        # When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val
        # FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed

        # Note: seq8() doesn't guarantee consecutive values in parallel execution,
        # so we use ROW_NUMBER() to ensure sequential integers.
        sql = (
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id, "
            f"TO_VARCHAR(ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::VARCHAR AS str_val "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY id"
        )
        rows = execute_query(sql)

        # Then there are 10000 rows returned
        assert len(rows) == LARGE_RESULT_SET_SIZE

        # And all returned string values should match the generated values in order
        assert_sequential_values(rows, LARGE_RESULT_SET_SIZE, transform=lambda i: (i, str(i)))
