"""Tests for Metadata/Catalog Operations.

Metadata inspection via cursor.description, SHOW, and DESCRIBE.
Used by SQLAlchemy for reflection, snowflake-cli for object management,
Snowfort for result classification.

Journey 9 - P1
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestMetadataCatalog:
    """Tests for metadata and catalog operations."""

    def test_should_return_metadata_via_show_tables(self, execute_query, cursor, tmp_schema):
        """Test SHOW TABLES returns metadata."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        # And A temporary table "meta_test" with diverse column types exists
        table_name = f"{tmp_schema}.meta_test"

        cursor.execute(
            f"CREATE TABLE {table_name} "
            f"(id INT, name VARCHAR(100), val NUMBER(10,2), "
            f"flag BOOLEAN, ts TIMESTAMP, data VARIANT)"
        )

        # When SHOW TABLES LIKE 'meta_test' is executed
        cursor.execute(f"SHOW TABLES LIKE 'meta_test' IN SCHEMA {tmp_schema}")
        results = cursor.fetchall()

        # Then 1 row should be returned with the table name
        assert len(results) == 1, f"Expected 1 table, got {len(results)}"
        # The table name appears in the result (typically in column index 1)
        row = results[0]
        assert "META_TEST" in str(row).upper(), f"Table name not found in result: {row}"

    def test_should_return_column_metadata_via_describe_table(self, execute_query, cursor, tmp_schema):
        """Test DESCRIBE TABLE returns column metadata."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        # And A temporary table "meta_test" with diverse column types exists
        table_name = f"{tmp_schema}.meta_test"

        cursor.execute(
            f"CREATE TABLE {table_name} "
            f"(id INT, name VARCHAR(100), val NUMBER(10,2), "
            f"flag BOOLEAN, ts TIMESTAMP, data VARIANT)"
        )

        # When DESCRIBE TABLE meta_test is executed
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

    def test_should_return_cursor_description_with_correct_column_metadata_after_select(
        self, execute_query, cursor, tmp_schema
    ):
        """Test cursor.description is populated after SELECT on table."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        # And A temporary table "meta_test" with diverse column types exists
        table_name = f"{tmp_schema}.meta_test"

        cursor.execute(
            f"CREATE TABLE {table_name} "
            f"(id INT, name VARCHAR(100), val NUMBER(10,2), "
            f"flag BOOLEAN, ts TIMESTAMP, data VARIANT)"
        )

        # When SELECT on the table is executed with WHERE 1=0
        cursor.execute(f"SELECT * FROM {table_name} WHERE 1=0")

        # Then cursor.description should have entries for each column with correct names
        assert cursor.description is not None, "cursor.description should not be None"
        assert len(cursor.description) == 6, f"Expected 6 columns, got {len(cursor.description)}"

        column_names = [desc[0] for desc in cursor.description]
        assert "ID" in column_names
        assert "NAME" in column_names
        assert "VAL" in column_names
        assert "FLAG" in column_names
        assert "TS" in column_names
        assert "DATA" in column_names

    def test_should_return_cursor_description_for_ad_hoc_select(self, execute_query, cursor):
        """Test cursor.description for ad-hoc SELECT."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When "SELECT 42 AS num, 'hello' AS str, TRUE AS flag" is executed
        cursor.execute("SELECT 42 AS num, 'hello' AS str, TRUE AS flag")

        # Then cursor.description should have 3 entries: NUM, STR, FLAG
        assert cursor.description is not None, "cursor.description should not be None"
        assert len(cursor.description) == 3, f"Expected 3 columns, got {len(cursor.description)}"

        column_names = [desc[0] for desc in cursor.description]
        assert column_names[0] == "NUM"
        assert column_names[1] == "STR"
        assert column_names[2] == "FLAG"

    def test_should_return_none_for_cursor_description_before_execute(self, execute_query, connection):
        """Test cursor.description is None before execute."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When A new cursor is created without executing any query
        with connection.cursor() as cursor:
            # Then cursor.description should be None
            assert cursor.description is None, "cursor.description should be None before execute"
