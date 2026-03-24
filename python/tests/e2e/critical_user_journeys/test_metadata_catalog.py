"""Tests for Metadata/Catalog Operations.

Metadata inspection via SHOW and DESCRIBE.
Used by SQLAlchemy for reflection, snowflake-cli for object management,
Snowfort for result classification.
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestMetadataCatalog:
    """Tests for metadata and catalog operations."""

    def test_should_return_metadata_via_show_tables(self, execute_query, cursor, tmp_schema):
        """Test SHOW TABLES returns metadata."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        # And A temporary table with diverse column types exists
        table_name = f"{tmp_schema}.meta_show"

        cursor.execute(
            f"CREATE OR REPLACE TABLE {table_name} "
            f"(id INT, name VARCHAR(100), val NUMBER(10,2), "
            f"flag BOOLEAN, ts TIMESTAMP, data VARIANT)"
        )

        # When SHOW TABLES LIKE 'meta_show' is executed
        cursor.execute(f"SHOW TABLES LIKE 'meta_show' IN SCHEMA {tmp_schema}")
        results = cursor.fetchall()

        # Then 1 row should be returned with the table name
        assert len(results) == 1, f"Expected 1 table, got {len(results)}"
        # The table name appears in the result (typically in column index 1)
        row = results[0]
        assert "META_SHOW" in str(row).upper(), f"Table name not found in result: {row}"

    def test_should_return_column_metadata_via_describe_table(self, execute_query, cursor, tmp_schema):
        """Test DESCRIBE TABLE returns column metadata."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        # And A temporary table with diverse column types exists
        table_name = f"{tmp_schema}.meta_describe"

        cursor.execute(
            f"CREATE OR REPLACE TABLE {table_name} "
            f"(id INT, name VARCHAR(100), val NUMBER(10,2), "
            f"flag BOOLEAN, ts TIMESTAMP, data VARIANT)"
        )

        # When DESCRIBE TABLE meta_describe is executed
        cursor.execute(f"DESCRIBE TABLE {table_name}")
        results = cursor.fetchall()

        # Then 6 rows should be returned with correct column names and types
        assert len(results) == 6, f"Expected 6 columns, got {len(results)}"

        # Verify column names (first column in DESCRIBE output)
        column_names = [row[0] for row in results]
        assert "ID" in column_names
        assert "NAME" in column_names
        assert "VAL" in column_names
        assert "FLAG" in column_names
        assert "TS" in column_names
        assert "DATA" in column_names
