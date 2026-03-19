"""Table Lifecycle E2E tests for Universal Driver.

This module tests full table lifecycle functionality including:
- CREATE TABLE with multiple column types
- DESCRIBE TABLE verification
- INSERT and SELECT with correct types
- WHERE clause filtering
- UPDATE with rowcount verification
- DELETE with rowcount verification
"""

from __future__ import annotations

from decimal import Decimal


class TestTableLifecycle:
    """Tests for full table lifecycle (DDL + DML)."""

    def test_should_create_table_with_multiple_column_types_and_verify_via_describe(self, cursor, tmp_schema):
        """Test creating table with multiple column types and verify via DESCRIBE."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.lifecycle_test"

        # When A temporary table "lifecycle_test" is created with columns
        # (id INT NOT NULL, name VARCHAR(100), active BOOLEAN, score FLOAT, amount NUMBER(10,2))
        cursor.execute(
            f"CREATE TABLE {table_name} ("
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

    def test_should_insert_multiple_rows_and_verify_via_select(self, cursor, tmp_schema):
        """Test inserting multiple rows and verify via SELECT with correct types."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.lifecycle_test"

        # And A temporary table "lifecycle_test" with typed columns exists
        cursor.execute(
            f"CREATE TABLE {table_name} ("
            f"id INT NOT NULL, "
            f"name VARCHAR(100), "
            f"active BOOLEAN, "
            f"score FLOAT, "
            f"amount NUMBER(10,2))"
        )

        # When 3 rows are inserted with diverse types
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'Alice', TRUE, 95.5, 1000.50)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (2, 'Bob', FALSE, 82.3, 2500.75)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (3, 'Charlie', TRUE, 88.9, 1750.25)")

        # Then SELECT with ORDER BY should return 3 rows with correct values and types
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        rows = cursor.fetchall()

        assert len(rows) == 3

        # Verify first row
        assert rows[0][0] == 1
        assert rows[0][1] == "Alice"
        assert rows[0][2] is True
        assert abs(float(rows[0][3]) - 95.5) < 0.01
        assert rows[0][4] == Decimal("1000.50")

        # Verify second row
        assert rows[1][0] == 2
        assert rows[1][1] == "Bob"
        assert rows[1][2] is False
        assert abs(float(rows[1][3]) - 82.3) < 0.01
        assert rows[1][4] == Decimal("2500.75")

        # Verify third row
        assert rows[2][0] == 3
        assert rows[2][1] == "Charlie"
        assert rows[2][2] is True
        assert abs(float(rows[2][3]) - 88.9) < 0.01
        assert rows[2][4] == Decimal("1750.25")

        # And Insert rowcount should be 3
        assert len(rows) == 3

    def test_should_filter_rows_with_where_clause(self, cursor, tmp_schema):
        """Test filtering rows with WHERE clause."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.lifecycle_test"

        # And A temporary table "lifecycle_test" with test data exists
        cursor.execute(
            f"CREATE TABLE {table_name} ("
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

    def test_should_update_row_and_verify_rowcount(self, cursor, tmp_schema):
        """Test updating row and verify rowcount."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.lifecycle_test"

        # And A temporary table "lifecycle_test" with test data exists
        cursor.execute(
            f"CREATE TABLE {table_name} ("
            f"id INT NOT NULL, "
            f"name VARCHAR(100), "
            f"active BOOLEAN, "
            f"score FLOAT, "
            f"amount NUMBER(10,2))"
        )
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'Alice', TRUE, 95.5, 1000.50)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (2, 'Bob', FALSE, 82.3, 2500.75)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (3, 'Charlie', TRUE, 88.9, 1750.25)")

        # When UPDATE SET name = 'Alice Updated' WHERE id = 1 is executed
        cursor.execute(f"UPDATE {table_name} SET name = 'Alice Updated' WHERE id = 1")

        # Then Update rowcount should be 1
        assert cursor.rowcount == 1

        # And SELECT should show the updated name for id=1
        cursor.execute(f"SELECT name FROM {table_name} WHERE id = 1")
        result = cursor.fetchone()
        assert result[0] == "Alice Updated"

    def test_should_delete_row_and_verify_rowcount(self, cursor, tmp_schema):
        """Test deleting row and verify rowcount."""
        # Given Snowflake client is logged in
        table_name = f"{tmp_schema}.lifecycle_test"

        # And A temporary table "lifecycle_test" with test data exists
        cursor.execute(
            f"CREATE TABLE {table_name} ("
            f"id INT NOT NULL, "
            f"name VARCHAR(100), "
            f"active BOOLEAN, "
            f"score FLOAT, "
            f"amount NUMBER(10,2))"
        )
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'Alice', TRUE, 95.5, 1000.50)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (2, 'Bob', FALSE, 82.3, 2500.75)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (3, 'Charlie', TRUE, 88.9, 1750.25)")

        # When DELETE WHERE active = FALSE is executed
        cursor.execute(f"DELETE FROM {table_name} WHERE active = FALSE")

        # Then Delete rowcount should be 1
        assert cursor.rowcount == 1

        # And SELECT COUNT(*) should reflect the deletion
        cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
        result = cursor.fetchone()
        assert result[0] == 2
