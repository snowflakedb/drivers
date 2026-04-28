"""Python-specific multistatement query tests (integration).

These tests cover Python-specific behavior not in the shared feature file:
- nextset() return values and behavior
- Cursor state management across nextset() calls
- Query ID tracking (multi_statement_savedIds and multi_statement_parent_sfqid)
- Mixed fetch modes (fetchall + nextset)
"""

from __future__ import annotations


class TestNextsetBehavior:
    """Tests for Python-specific nextset() behavior."""

    def test_nextset_returns_none_without_multistatement(self, cursor):
        """Test that nextset returns None for single-statement queries."""
        # Execute single statement
        cursor.execute("SELECT 1")
        row = cursor.fetchone()
        assert row is not None
        assert row[0] == 1

        # nextset should return None
        result = cursor.nextset()
        assert result is None

    def test_nextset_resets_cursor_state(self, cursor):
        """Test that nextset properly resets cursor state."""
        cursor.execute("SELECT 1, 2; SELECT 3, 4, 5", num_statements=2)

        # First result: 2 columns
        assert cursor.description is not None
        assert len(cursor.description) == 2
        cursor.fetchone()

        # After nextset, description should reflect new columns
        cursor.nextset()
        assert cursor.description is not None
        assert len(cursor.description) == 3

        # rownumber should be reset
        assert cursor.rownumber is None or cursor.rownumber == -1

    def test_parent_query_id_tracking(self, cursor):
        """Test that query IDs are tracked for multi-statement."""
        cursor.execute("SELECT 1; SELECT 2", num_statements=2)

        # Verify multi_statement_savedIds contains child query IDs
        assert hasattr(cursor, "multi_statement_savedIds")
        assert len(cursor.multi_statement_savedIds) == 2

        # Verify multi_statement_parent_sfqid if available
        if hasattr(cursor, "multi_statement_parent_sfqid"):
            parent_qid = cursor.multi_statement_parent_sfqid
            # Parent QID should persist across nextset
            cursor.nextset()
            assert cursor.multi_statement_parent_sfqid == parent_qid

    def test_fetchall_then_nextset(self, cursor):
        """Test that fetchall works before nextset."""
        cursor.execute("SELECT 1 UNION ALL SELECT 2; SELECT 3", num_statements=2)

        # Fetch all from first result
        rows = cursor.fetchall()
        assert len(rows) == 2
        assert rows[0][0] == 1
        assert rows[1][0] == 2

        # Move to second result
        cursor.nextset()
        row = cursor.fetchone()
        assert row is not None
        assert row[0] == 3

    def test_parameter_persistence(self, cursor):
        """Test that num_statements parameter works across multiple executions."""
        # First execution
        cursor.execute("SELECT 1; SELECT 2", num_statements=2)
        cursor.fetchone()
        cursor.nextset()
        cursor.fetchone()
        assert cursor.nextset() is None

        # Second execution
        cursor.execute("SELECT 3; SELECT 4", num_statements=2)
        cursor.fetchone()
        result = cursor.nextset()
        assert result is cursor
        cursor.fetchone()
        assert cursor.nextset() is None


class TestResultBatchesWithMultiStatement:
    """Tests for get_result_batches() across multi-statement child results."""

    def test_should_return_batches_for_each_child_after_nextset(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When A multi-statement query with two large SELECTs is executed
        first_stmt_row_count = 200_001
        second_stmt_row_count = 200_002
        cursor.execute(
            f"SELECT seq4() AS id FROM TABLE(GENERATOR(ROWCOUNT => {first_stmt_row_count})) v; "
            f"SELECT seq4() AS id FROM TABLE(GENERATOR(ROWCOUNT => {second_stmt_row_count})) v",
            num_statements=2,
        )

        # Then get_result_batches returns multiple batches for the first child
        first_batches = cursor.get_result_batches()
        assert first_batches is not None
        assert len(first_batches) >= 2, "Expected at least an inline batch and one remote batch"
        assert sum(b.rowcount for b in first_batches) == first_stmt_row_count

        # When Advancing to the second result set
        result = cursor.nextset()
        assert result is not None

        # Then get_result_batches returns multiple batches for the second child
        second_batches = cursor.get_result_batches()
        assert second_batches is not None
        assert len(second_batches) >= 2, "Expected at least an inline batch and one remote batch"
        assert sum(b.rowcount for b in second_batches) == second_stmt_row_count
