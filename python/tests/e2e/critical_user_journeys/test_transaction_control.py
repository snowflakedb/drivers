"""Transaction Control E2E tests for Universal Driver.

This module tests transaction management functionality including:
- Autocommit toggling
- COMMIT operations
- ROLLBACK operations
"""

from __future__ import annotations


class TestTransactionControl:
    """Tests for transaction control (commit, rollback, autocommit)."""

    def test_should_rollback_uncommitted_insert_when_autocommit_is_off(self, connection, cursor, tmp_schema):
        """Test rollback of uncommitted insert when autocommit is OFF."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.tx_test"

        # And A test table "tx_test" with columns (id INT, name VARCHAR) exists
        cursor.execute(f"CREATE TABLE {table_name} (id INT, name VARCHAR)")

        # And Autocommit is set to OFF
        connection.autocommit(False)

        # When A row (1, 'uncommitted') is inserted into "tx_test"
        cursor.execute(f"INSERT INTO {table_name} (id, name) VALUES (1, 'uncommitted')")

        # And The row is visible within the session via SELECT
        cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
        assert cursor.fetchone()[0] == 1

        # And ROLLBACK is executed
        connection.rollback()

        # Then The table "tx_test" should have 0 rows
        cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
        assert cursor.fetchone()[0] == 0

    def test_should_persist_committed_insert_when_autocommit_is_off(self, connection, cursor, tmp_schema):
        """Test that committed insert persists when autocommit is OFF."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.tx_test"

        # And A test table "tx_test" with columns (id INT, name VARCHAR) exists
        cursor.execute(f"CREATE TABLE {table_name} (id INT, name VARCHAR)")

        # And Autocommit is set to OFF
        connection.autocommit(False)

        # When A row (2, 'committed') is inserted into "tx_test"
        cursor.execute(f"INSERT INTO {table_name} (id, name) VALUES (2, 'committed')")

        # And COMMIT is executed
        connection.commit()

        # Then The table "tx_test" should have 1 row with id=2
        cursor.execute(f"SELECT id, name FROM {table_name} WHERE id = 2")
        result = cursor.fetchone()
        assert result is not None
        assert result[0] == 2
        assert result[1] == "committed"

    def test_should_auto_persist_insert_when_autocommit_is_on(self, connection, cursor, tmp_schema):
        """Test that insert auto-persists when autocommit is ON."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.tx_test"

        # And A test table "tx_test" with columns (id INT, name VARCHAR) exists
        cursor.execute(f"CREATE TABLE {table_name} (id INT, name VARCHAR)")

        # And Autocommit is set to ON
        connection.autocommit(True)

        # When A row (3, 'auto') is inserted into "tx_test"
        cursor.execute(f"INSERT INTO {table_name} (id, name) VALUES (3, 'auto')")

        # Then The row should be immediately visible without explicit commit
        cursor.execute(f"SELECT id, name FROM {table_name} WHERE id = 3")
        result = cursor.fetchone()
        assert result is not None
        assert result[0] == 3
        assert result[1] == "auto"
