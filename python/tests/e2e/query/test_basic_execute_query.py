"""Basic execute query tests for Universal Driver (Python-specific).

This module tests basic query execution functionality including:
- Simple SELECT queries returning single value
- SELECT queries returning multiple columns
- SELECT queries returning multiple rows
- SELECT queries returning empty result set
- SELECT queries returning NULL values
- DDL statements (CREATE and DROP TABLE)
- DML statements (INSERT and SELECT)
- Error handling for invalid SQL
- Sequential query execution on same connection
"""

from __future__ import annotations

import pytest

from tests.e2e.types.utils import assert_sequential_values

from ...compatibility import IS_UNIVERSAL_DRIVER


class TestSelectQueries:
    """Tests for basic SELECT query execution."""

    def test_should_execute_simple_select_returning_single_value(self, cursor):
        """Test simple SELECT returning single value."""
        # Given Snowflake client is logged in

        # When Query "SELECT 1 AS value" is executed
        cursor.execute("SELECT 1 AS value")
        result = cursor.fetchone()

        # Then the result should contain value 1
        assert result is not None
        assert result[0] == 1

    def test_should_execute_select_returning_multiple_columns(self, cursor):
        """Test SELECT returning multiple columns."""
        # Given Snowflake client is logged in

        # When Query "SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3" is executed
        cursor.execute("SELECT 1 AS col1, 'hello' AS col2, '3.14' AS col3")
        result = cursor.fetchone()

        # Then the result should contain:

        #   | col1 | col2  | col3 |
        #   | 1    | hello | 3.14 |
        assert result is not None
        assert result[0] == 1
        assert result[1] == "hello"
        assert result[2] == "3.14"

    def test_should_execute_select_returning_multiple_rows(self, cursor):
        """Test SELECT returning multiple rows using GENERATOR."""
        # Given Snowflake client is logged in

        # When Query "SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id" is executed
        cursor.execute("SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) v ORDER BY id")
        rows = cursor.fetchall()

        # Then there are 5 numbered sequentially rows returned
        assert len(rows) == 5
        values = [row[0] for row in rows]
        assert_sequential_values(values, 5)

    @pytest.mark.skip_universal(reason="Known issue SNOW-2997744: Empty result set handling")
    def test_should_execute_select_returning_empty_result_set(self, cursor):
        """Test SELECT returning empty result set."""
        # Given Snowflake client is logged in

        # When Query "SELECT 1 WHERE 1=0" is executed
        cursor.execute("SELECT 1 WHERE 1=0")
        result = cursor.fetchall()

        # Then the result set should be empty
        assert result == []

    def test_should_execute_select_returning_null_values(self, cursor):
        """Test SELECT returning NULL values."""
        # Given Snowflake client is logged in

        # When Query "SELECT NULL AS col1, 42 AS col2, NULL AS col3" is executed
        cursor.execute("SELECT NULL AS col1, 42 AS col2, NULL AS col3")
        result = cursor.fetchone()

        # Then the result should contain NULL for col1 and col3 and 42 for col2
        assert result is not None
        assert result[0] is None
        assert result[1] == 42
        assert result[2] is None


class TestDDLStatements:
    """Tests for DDL (Data Definition Language) statements."""

    def test_should_execute_create_and_drop_table_statements(self, cursor, tmp_schema):
        """Test CREATE and DROP TABLE statements."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.test_basic_ddl"

        # When CREATE TABLE statement is executed
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # Then the table should be created successfully
        cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
        result = cursor.fetchone()
        assert result[0] == 0

        # And DROP TABLE statement should complete successfully
        cursor.execute(f"DROP TABLE {table_name}")
        result = cursor.fetchone()
        assert "successfully dropped" in result[0]


class TestDMLStatements:
    """Tests for DML (Data Manipulation Language) statements."""

    def test_should_execute_insert_and_retrieve_inserted_data(self, cursor, tmp_schema):
        """Test INSERT and retrieve inserted data."""
        # Given Snowflake client is logged in

        # And A temporary table is created
        table_name = f"{tmp_schema}.test_basic_dml"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, value VARCHAR)")

        # When INSERT statement is executed to add rows
        cursor.execute(f"INSERT INTO {table_name} (id, value) VALUES (1, 'first')")
        cursor.execute(f"INSERT INTO {table_name} (id, value) VALUES (2, 'second')")
        cursor.execute(f"INSERT INTO {table_name} (id, value) VALUES (3, 'third')")

        # And Query "SELECT id, value FROM {table} ORDER BY id" is executed
        cursor.execute(f"SELECT id, value FROM {table_name} ORDER BY id")
        results = cursor.fetchall()

        # Then the inserted data should be correctly returned
        assert len(results) == 3
        assert results[0] == (1, "first")
        assert results[1] == (2, "second")
        assert results[2] == (3, "third")


class TestErrorHandling:
    """Tests for error handling."""

    def test_should_return_error_for_invalid_sql_syntax(self, cursor):
        """Test error handling for invalid SQL syntax."""
        # Given Snowflake client is logged in

        # When Invalid SQL "SELCT INVALID SYNTAX" is executed
        # Then An error should be returned
        if IS_UNIVERSAL_DRIVER:
            # TODO: this is not a desired state. Error type should match after error unification PR.
            from snowflake.connector._internal.protobuf_gen.proto_exception import ProtoApplicationException

            expected_error = ProtoApplicationException
        else:
            from snowflake.connector import ProgrammingError

            expected_error = ProgrammingError

        with pytest.raises(expected_error):
            cursor.execute("SELCT INVALID SYNTAX")


class TestSequentialExecution:
    """Tests for sequential query execution."""

    def test_should_execute_multiple_queries_sequentially_on_same_connection(self, cursor):
        """Test multiple queries executed sequentially on same connection."""
        # Given Snowflake client is logged in

        # When Multiple queries are executed sequentially
        cursor.execute("SELECT 1 AS first_query")
        result1 = cursor.fetchone()

        cursor.execute("SELECT 'hello' AS second_query")
        result2 = cursor.fetchone()

        cursor.execute("SELECT 42, 'world' AS third_query")
        result3 = cursor.fetchone()

        # Then each query should return correct results
        assert result1 is not None
        assert result1[0] == 1

        assert result2 is not None
        assert result2[0] == "hello"

        assert result3 is not None
        assert result3[0] == 42
        assert result3[1] == "world"
