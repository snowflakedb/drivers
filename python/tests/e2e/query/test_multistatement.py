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

from ...conftest import with_paramstyle


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


@with_paramstyle("qmark")
class TestMultistatementWithParameters:
    """Tests for multistatement queries combined with positional parameter binding."""

    def test_should_execute_multistatement_dml_with_positional_parameters(self, cursor):
        # Given Snowflake client is logged in
        pass
        # And A temporary table with column (id NUMBER) exists
        table_name = random_table_name("ms_bind_dml")
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table_name}(id NUMBER)")

        # When Multistatement INSERT chain is executed with 3 positional parameters
        cursor.execute(
            f"INSERT INTO {table_name} VALUES(?); INSERT INTO {table_name} VALUES(?),(?)",
            (10, 20, 30),
            num_statements=2,
        )

        # Then 2 result sets are returned
        pass
        # And the first result set reports update count 1
        assert cursor.rowcount == 1

        # And the second result set reports update count 2
        assert cursor.nextset() is cursor
        assert cursor.rowcount == 2
        assert cursor.nextset() is None

        # And the table contains rows [10, 20, 30]
        with cursor.connection.cursor() as verify:
            verify.execute(f"SELECT id FROM {table_name} ORDER BY id")
            assert verify.fetchall() == [(10,), (20,), (30,)]

    def test_should_execute_multistatement_select_with_positional_parameters(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When Multistatement SELECT chain is executed with 6 positional parameters
        cursor.execute(
            "SELECT ?; SELECT ?, ?; SELECT ?, ?, ?",
            (10, 20, 30, 40, 50, 60),
            num_statements=3,
        )

        # Then 3 result sets are returned
        pass
        # And the first result set contains row [10]
        assert cursor.fetchone() == (10,)

        # And the second result set contains row [20, 30]
        assert cursor.nextset() is cursor
        assert cursor.fetchone() == (20, 30)

        # And the third result set contains row [40, 50, 60]
        assert cursor.nextset() is cursor
        assert cursor.fetchone() == (40, 50, 60)
        assert cursor.nextset() is None

    def test_should_fail_when_multistatement_query_has_too_few_parameters(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When Multistatement SELECT requires 3 parameters but only 1 is bound
        pass

        # Then an error is returned indicating parameter count mismatch
        with pytest.raises(ProgrammingError):
            cursor.execute(
                "SELECT ?; SELECT ?, ?",
                (10,),
                num_statements=2,
            )

    def test_should_fail_when_null_positional_parameters_are_used_in_multistatement_query(self, cursor):
        # Given Snowflake client is logged in
        pass
        # When Multistatement SELECT is executed with NULL positional parameters
        pass

        # Then an error is returned indicating NULL bindings are not supported
        with pytest.raises(ProgrammingError, match=r"(?i)bind"):
            cursor.execute(
                "SELECT ?; SELECT ?, ?",
                (None, 10, None),
                num_statements=2,
            )
