"""Table Lifecycle E2E tests for Universal Driver.

This module tests full table lifecycle functionality including:
- CREATE TABLE with multiple column types
- DESCRIBE TABLE verification
- WHERE clause filtering
"""

from __future__ import annotations


class TestTableLifecycle:
    """Tests for full table lifecycle (DDL + DML)."""

    def test_should_create_table_with_multiple_column_types_and_verify_via_describe(self, cursor, tmp_schema):
        """Test creating table with multiple column types and verify via DESCRIBE."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.lifecycle_describe"

        # When A temporary table is created with columns
        # (id INT NOT NULL, name VARCHAR(100), active BOOLEAN, score FLOAT, amount NUMBER(10,2))
        cursor.execute(
            f"CREATE OR REPLACE TABLE {table_name} ("
            f"id INT NOT NULL, "
            f"name VARCHAR(100), "
            f"active BOOLEAN, "
            f"score FLOAT, "
            f"amount NUMBER(10,2))"
        )

        # Then DESCRIBE TABLE should return 5 columns with correct names and types
        cursor.execute(f"DESCRIBE TABLE {table_name}")
        columns = cursor.fetchall()

        assert len(columns) == 5

        # Verify column names
        column_names = [col[0] for col in columns]
        assert "ID" in column_names
        assert "NAME" in column_names
        assert "ACTIVE" in column_names
        assert "SCORE" in column_names
        assert "AMOUNT" in column_names

        # Verify column types
        column_types = {col[0]: col[1] for col in columns}
        assert "NUMBER" in column_types["ID"]
        assert "VARCHAR" in column_types["NAME"]
        assert "BOOLEAN" in column_types["ACTIVE"]
        assert "FLOAT" in column_types["SCORE"]
        assert "NUMBER" in column_types["AMOUNT"]

    def test_should_filter_rows_with_where_clause(self, cursor, tmp_schema):
        """Test filtering rows with WHERE clause."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.lifecycle_filter"

        # And A temporary table with test data exists
        cursor.execute(
            f"CREATE OR REPLACE TABLE {table_name} ("
            f"id INT NOT NULL, "
            f"name VARCHAR(100), "
            f"active BOOLEAN, "
            f"score FLOAT, "
            f"amount NUMBER(10,2))"
        )
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'Alice', TRUE, 95.5, 1000.50)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (2, 'Bob', FALSE, 82.3, 2500.75)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (3, 'Charlie', TRUE, 88.9, 1750.25)")

        # When SELECT with WHERE active = TRUE is executed
        cursor.execute(f"SELECT id, name, active FROM {table_name} WHERE active = TRUE ORDER BY id")
        rows = cursor.fetchall()

        # Then Only rows where active is TRUE should be returned
        assert len(rows) == 2
        assert rows[0][0] == 1
        assert rows[0][1] == "Alice"
        assert rows[0][2] is True
        assert rows[1][0] == 3
        assert rows[1][1] == "Charlie"
        assert rows[1][2] is True
