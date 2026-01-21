"""BOOLEAN type tests for Universal Driver."""

from __future__ import annotations

import pytest

from .utils import assert_type


# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE_GENERATOR = 1_000_000
LARGE_RESULT_SET_SIZE_TABLE = 1_000_000


class TestBooleanTypeCasting:
    """Tests for BOOLEAN type casting to appropriate type."""

    def test_should_cast_boolean_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
        result = execute_query("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN", single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, bool)

        # And Values should match [TRUE, FALSE, TRUE]
        assert result == (True, False, True)


class TestBooleanLiteral:
    """Tests for BOOLEAN type using SELECT with literals (no tables)."""

    def test_should_select_boolean_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
        result = execute_query("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN", single_row=True)

        # Then Result should contain [TRUE, FALSE]
        assert result == (True, False)
        assert_type(result, bool)

    def test_should_handle_null_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
        result = execute_query("SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN", single_row=True)

        # Then Result should contain [FALSE, NULL, TRUE, NULL]
        assert result == (False, None, True, None)
        assert_type(result, bool, can_be_none=True)

    def test_should_download_large_result_set_with_multiple_chunks_from_generator(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT (seq8() % 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v" is executed
        sql = f"SELECT (seq8() % 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE_GENERATOR})) v"
        rows = execute_query(sql)
        result = [row[0] for row in rows]

        # Then Result should contain 1000000 alternating TRUE and FALSE values
        assert_type(result, bool)
        assert result == [True, False] * (LARGE_RESULT_SET_SIZE_GENERATOR // 2)


class TestBooleanTable:
    """Tests for BOOLEAN type using table operations."""

    def test_should_select_boolean_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
        table_name = f"{tmp_schema}.boolean_table"
        execute_query(f"CREATE TABLE {table_name} (col1 BOOLEAN, col2 BOOLEAN, col3 BOOLEAN)")

        # And Row (TRUE, FALSE, TRUE) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (TRUE, FALSE, TRUE)")

        # When Query "SELECT * FROM <table>" is executed
        result = execute_query(f"SELECT * FROM {table_name}", single_row=True)

        # Then Result should contain [TRUE, FALSE, TRUE]
        assert_type(result, bool)
        assert result == (True, False, True)

    def test_should_handle_null_values_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with BOOLEAN column exists
        table_name = f"{tmp_schema}.null_table"
        execute_query(f"CREATE TABLE {table_name} (col BOOLEAN)")

        # And Rows [NULL, TRUE, FALSE] are inserted
        execute_query(f"INSERT INTO {table_name} VALUES (NULL), (TRUE), (FALSE)")

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain [NULL, TRUE, FALSE]
        result = [row[0] for row in rows]
        assert result == [None, True, False]
        assert_type(result, bool, can_be_none=True)

    def test_should_download_large_result_set_with_multiple_chunks_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with BOOLEAN column exists with 1000000 alternating boolean values
        table_name = f"{tmp_schema}.large_boolean_table"
        execute_query(f"CREATE TABLE {table_name} (col BOOLEAN)")
        execute_query(
            f"INSERT INTO {table_name} SELECT (seq8() % 2 = 0)::BOOLEAN "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE_TABLE}))"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")
        result = [row[0] for row in rows]

        # Then Result should contain 1000000 alternating boolean values
        assert_type(result, bool)
        assert result == [True, False] * (LARGE_RESULT_SET_SIZE_GENERATOR // 2)


class TestBooleanBinding:
    """Tests for BOOLEAN type using parameter binding."""

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    def test_should_select_boolean_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed
        # with bound boolean values [TRUE, FALSE, TRUE]
        result = execute_query("SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN", (True, False, True), single_row=True)

        # Then Result should contain [TRUE, FALSE, TRUE]
        assert result == (True, False, True)
        assert_type(result, bool)

        # When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
        result = execute_query("SELECT ?::BOOLEAN", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    def test_should_insert_boolean_using_parameter_binding(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with BOOLEAN column exists
        table_name = f"{tmp_schema}.boolean_bind_table"
        execute_query(f"CREATE TABLE {table_name} (col BOOLEAN)")

        # When Boolean values [TRUE, FALSE, NULL] are inserted using binding
        test_values = [True, False, None]
        for val in test_values:
            execute_query(f"INSERT INTO {table_name} VALUES (?)", (val,))

        # Then SELECT should return the same exact values
        rows = execute_query(f"SELECT * FROM {table_name}")
        result = [row[0] for row in rows]
        assert result == test_values
        assert_type(result, bool, can_be_none=True)
