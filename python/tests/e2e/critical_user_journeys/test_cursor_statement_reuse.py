"""Tests for Cursor/Statement Reuse.

Cursor reuse across multiple queries with state replacement and error recovery.
Used by Snowfort for thousands of sequential queries, Snowpark for DataFrame
operations, CLI for interactive REPL sessions.

Journey 14 - P1
"""

from __future__ import annotations

import pytest

from snowflake.connector import ProgrammingError
from tests.e2e.types.utils import assert_connection_is_open


class TestCursorStatementReuse:
    """Tests for cursor reuse across multiple operations."""

    def test_should_replace_cursor_state_on_subsequent_queries(self, execute_query, cursor, tmp_schema):
        """Test cursor state is replaced on subsequent queries."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A cursor is created
        assert cursor is not None

        # When "SELECT 1 AS a" is executed
        cursor.execute("SELECT 1 AS a")
        result1 = cursor.fetchone()

        # Then The cursor should have 1 column named "A"
        assert cursor.description is not None
        assert len(cursor.description) == 1, f"Expected 1 column, got {len(cursor.description)}"
        assert cursor.description[0][0] == "A"
        assert result1[0] == 1

        # Save the sfqid from first query
        sfqid1 = cursor.sfqid

        # When "SELECT 2 AS b, 3 AS c" is executed on the same cursor
        cursor.execute("SELECT 2 AS b, 3 AS c")
        result2 = cursor.fetchone()

        # Then The cursor should have 2 columns named "B" and "C"
        assert cursor.description is not None
        assert len(cursor.description) == 2, f"Expected 2 columns, got {len(cursor.description)}"
        assert cursor.description[0][0] == "B"
        assert cursor.description[1][0] == "C"
        assert result2 == (2, 3)

        # And sfqid should have changed
        sfqid2 = cursor.sfqid
        assert sfqid1 != sfqid2, "sfqid should change between queries"

    def test_should_reuse_cursor_across_ddl_dml_and_select(self, execute_query, cursor, tmp_schema):
        """Test cursor reuse across DDL, DML, and SELECT."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A cursor is created
        assert cursor is not None
        table_name = f"{tmp_schema}.reuse_test"

        # When CREATE TEMPORARY TABLE is executed on the cursor
        cursor.execute(f"CREATE TABLE {table_name} (id INT, value VARCHAR)")

        # And INSERT is executed on the same cursor
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'test')")
        assert cursor.rowcount == 1, "INSERT should affect 1 row"

        # And SELECT is executed on the same cursor
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()

        # Then Each operation should succeed with correct results
        assert result == (1, "test"), f"SELECT should return (1, 'test'), got {result}"

    def test_should_recover_cursor_after_error_and_execute_successfully(self, execute_query, cursor):
        """Test cursor recovers after error and executes successfully."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A cursor is created
        assert cursor is not None

        # When Invalid SQL is executed and the error is caught
        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL SYNTAX HERE")

        # And "SELECT 42" is executed on the same cursor
        cursor.execute("SELECT 42")
        result = cursor.fetchone()

        # Then The cursor should return (42,) successfully
        assert result == (42,), f"Expected (42,), got {result}"
