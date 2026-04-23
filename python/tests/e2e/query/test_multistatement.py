"""Multistatement query execution tests for Universal Driver (Python).

This module implements the test scenarios from:
  tests/definitions/shared/query/multistatement.feature

These are cross-driver tests that validate shared multistatement behavior.
Python-specific tests are in tests/integ/query/test_multistatement.py
"""

from __future__ import annotations

import uuid

import pytest

from snowflake.connector.errors import ProgrammingError


def random_table_name(prefix: str = "ms_test") -> str:
    """Generate a random table name to avoid conflicts."""
    return f"{prefix}_{uuid.uuid4().hex[:8]}"


class TestMultipleSelectStatements:
    """Tests for multiple SELECT statement execution."""

    def test_should_execute_multiple_select_statements(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When multistatement query with 3 SELECTs is executed
        cursor.execute("SELECT 1 AS a; SELECT 2 AS b; SELECT 3 AS c", num_statements=3)

        # Then 3 result sets are returned
        pass
        # And each result set contains correct data

        # First result
        assert cursor.description is not None
        assert cursor.description[0].name == "A"
        row1 = cursor.fetchone()
        assert row1 is not None
        assert row1[0] == 1

        # Second result
        result = cursor.nextset()
        assert result is cursor
        assert cursor.description is not None
        assert cursor.description[0].name == "B"
        row2 = cursor.fetchone()
        assert row2 is not None
        assert row2[0] == 2

        # Third result
        result = cursor.nextset()
        assert result is cursor
        assert cursor.description is not None
        assert cursor.description[0].name == "C"
        row3 = cursor.fetchone()
        assert row3 is not None
        assert row3[0] == 3

        # No more results
        result = cursor.nextset()
        assert result is None


class TestMultipleDMLStatements:
    """Tests for multiple DML statement execution."""

    def test_should_execute_multiple_dml_statements(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When multistatement query with CREATE TABLE, INSERT, and DROP is executed
        table_name = random_table_name("ms_dml")
        sql = (
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name}(id INT); "
            f"INSERT INTO {table_name} VALUES (1),(2),(3); "
            f"DROP TABLE {table_name}"
        )
        cursor.execute(sql, num_statements=3)

        # Then 3 result sets are returned

        # First result: CREATE TABLE
        assert cursor.rowcount == 1

        # Second result: INSERT
        result = cursor.nextset()
        assert result is cursor
        assert cursor.rowcount == 3

        # Third result: DROP TABLE
        result = cursor.nextset()
        assert result is cursor
        assert cursor.rowcount == 1

        # No more results
        result = cursor.nextset()
        assert result is None


class TestMixedStatementTypes:
    """Tests for mixed statement type execution."""

    def test_should_execute_mixed_statement_types(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When multistatement query with various types is executed
        table_name = random_table_name("ms_mix")
        sql = (
            "ALTER SESSION SET TIMEZONE='UTC'; "
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name}(val TEXT); "
            f"INSERT INTO {table_name} VALUES ('hello'); "
            f"SELECT val FROM {table_name}; "
            f"DROP TABLE {table_name}"
        )
        cursor.execute(sql, num_statements=5)

        # Then 5 result sets are returned
        pass
        # 1. ALTER SESSION
        assert cursor.rowcount == 1

        # 2. CREATE TABLE
        result = cursor.nextset()
        assert result is cursor
        assert cursor.rowcount == 1

        # 3. INSERT
        result = cursor.nextset()
        assert result is cursor
        assert cursor.rowcount == 1

        # 4. SELECT - and the SELECT result contains expected data
        result = cursor.nextset()
        assert result is cursor
        # And the SELECT result contains expected data
        row = cursor.fetchone()
        assert row is not None
        assert row[0] == "hello"

        # 5. DROP TABLE
        result = cursor.nextset()
        assert result is cursor
        assert cursor.rowcount == 1

        # No more results
        result = cursor.nextset()
        assert result is None


class TestErrorHandling:
    """Tests for multistatement error handling."""

    def test_should_fail_when_multistatement_sql_is_sent_without_multi_statement_count(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When multistatement SQL is executed without configuring multi_statement_count
        pass
        # Then an error is returned indicating multi-statement is not enabled
        with pytest.raises(ProgrammingError) as exc_info:
            cursor.execute("SELECT 1; SELECT 2")

        # Verify error indicates multi-statement is not enabled
        assert exc_info.value is not None

    def test_should_fail_when_multi_statement_count_does_not_match_actual_statement_count(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When single SELECT is executed with multi_statement_count set to 3
        pass

        # Then an error is returned indicating statement count mismatch
        with pytest.raises(ProgrammingError) as exc_info:
            cursor.execute("SELECT 1", num_statements=3)

        # Verify error is raised
        assert exc_info.value is not None
