"""Tests for Object Discovery (SHOW/DESCRIBE/DROP).

Object management via SHOW, DESCRIBE, and DROP commands.
Used by snowflake-cli for all object management, SQLAlchemy for metadata
reflection, Snowfort extensively.
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestObjectDiscovery:
    """Tests for object discovery operations."""

    def test_should_discover_table_via_show_and_describe(self, execute_query, cursor, tmp_schema):
        """Test table discovery via SHOW TABLES and DESCRIBE TABLE."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A table with columns (id INT NOT NULL, name VARCHAR(100), val NUMBER(10,2)) exists
        table_name = f"{tmp_schema}.disc_show_describe"

        cursor.execute(f"CREATE OR REPLACE TABLE {table_name} (id INT NOT NULL, name VARCHAR(100), val NUMBER(10,2))")

        # When SHOW TABLES LIKE 'disc_show_describe' is executed
        cursor.execute(f"SHOW TABLES LIKE 'disc_show_describe' IN SCHEMA {tmp_schema}")
        show_results = cursor.fetchall()

        # Then 1 row should be returned
        assert len(show_results) == 1, f"Expected 1 table, got {len(show_results)}"

        # When DESCRIBE TABLE e2e_discovery_test is executed
        cursor.execute(f"DESCRIBE TABLE {table_name}")
        describe_results = cursor.fetchall()

        # Then 3 columns with correct names and types should be returned
        assert len(describe_results) == 3, f"Expected 3 columns, got {len(describe_results)}"

        # Verify column names (first column in DESCRIBE output)
        column_names = [row[0] for row in describe_results]
        assert "ID" in column_names
        assert "NAME" in column_names
        assert "VAL" in column_names

    def test_should_discover_view_via_show(self, execute_query, cursor, tmp_schema):
        """Test view discovery via SHOW VIEWS."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A table exists
        table_name = f"{tmp_schema}.disc_view_base"
        view_name = f"{tmp_schema}.disc_view_show"

        cursor.execute(f"CREATE OR REPLACE TABLE {table_name} (id INT, name VARCHAR)")

        # And A view is created on the table
        cursor.execute(f"CREATE OR REPLACE VIEW {view_name} AS SELECT * FROM {table_name}")

        # When SHOW VIEWS LIKE 'disc_view_show' is executed
        cursor.execute(f"SHOW VIEWS LIKE 'disc_view_show' IN SCHEMA {tmp_schema}")
        show_results = cursor.fetchall()

        # Then 1 row should be returned
        assert len(show_results) == 1, f"Expected 1 view, got {len(show_results)}"

    def test_should_verify_objects_are_gone_after_drop(self, execute_query, cursor, tmp_schema):
        """Test that objects are not discoverable after DROP."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A table and view exist
        table_name = f"{tmp_schema}.disc_drop_table"
        view_name = f"{tmp_schema}.disc_drop_view"

        cursor.execute(f"CREATE OR REPLACE TABLE {table_name} (id INT, name VARCHAR)")
        cursor.execute(f"CREATE OR REPLACE VIEW {view_name} AS SELECT * FROM {table_name}")

        # When Both objects are dropped
        cursor.execute(f"DROP VIEW {view_name}")
        cursor.execute(f"DROP TABLE {table_name}")

        # Then SHOW TABLES should return 0 rows
        cursor.execute(f"SHOW TABLES LIKE 'disc_drop_table' IN SCHEMA {tmp_schema}")
        table_results = cursor.fetchall()
        assert len(table_results) == 0, f"Expected 0 tables after drop, got {len(table_results)}"

        # And SHOW VIEWS should return 0 rows
        cursor.execute(f"SHOW VIEWS LIKE 'disc_drop_view' IN SCHEMA {tmp_schema}")
        view_results = cursor.fetchall()
        assert len(view_results) == 0, f"Expected 0 views after drop, got {len(view_results)}"
