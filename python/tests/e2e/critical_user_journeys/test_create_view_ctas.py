"""Tests for CREATE VIEW / CREATE TABLE AS SELECT.

View creation and CTAS operations.
Used by Snowpark for create_or_replace_view() and save_as_table(),
SQLAlchemy for views.

Journey 24 - P2
"""

from __future__ import annotations

import pytest

from tests.e2e.types.utils import assert_connection_is_open


class TestCreateViewCtas:
    """Tests for view creation and CTAS operations."""

    def test_should_create_view_and_query_filtered_data(self, execute_query, cursor, tmp_schema):
        """Test CREATE VIEW and query filtered data."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A source table with 3 rows of test data exists
        source_table = f"{tmp_schema}.ctas_source"
        view_name = f"{tmp_schema}.ctas_view"

        cursor.execute(f"CREATE TABLE {source_table} (id INT, name VARCHAR, val FLOAT)")
        cursor.execute(f"INSERT INTO {source_table} VALUES (1, 'first', 1.0), (2, 'second', 2.0), (3, 'third', 3.0)")

        # When A view is created that filters rows where id > 1
        cursor.execute(f"CREATE VIEW {view_name} AS SELECT * FROM {source_table} WHERE id > 1")

        # Then SELECT from the view should return 2 rows
        cursor.execute(f"SELECT * FROM {view_name} ORDER BY id")
        rows = cursor.fetchall()

        assert len(rows) == 2
        assert rows[0][0] == 2
        assert rows[1][0] == 3

    def test_should_create_table_as_select(self, execute_query, cursor, tmp_schema):
        """Test CREATE TABLE AS SELECT (CTAS)."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A source table with 3 rows of test data exists
        source_table = f"{tmp_schema}.ctas_source"
        new_table = f"{tmp_schema}.ctas_result"

        cursor.execute(f"CREATE TABLE {source_table} (id INT, name VARCHAR, val FLOAT)")
        cursor.execute(f"INSERT INTO {source_table} VALUES (1, 'first', 1.5), (2, 'second', 2.5), (3, 'third', 3.5)")

        # When CREATE TABLE AS SELECT is executed filtering val > 2.0
        cursor.execute(f"CREATE TABLE {new_table} AS SELECT * FROM {source_table} WHERE val > 2.0")

        # Then The new table should contain the filtered rows
        cursor.execute(f"SELECT * FROM {new_table} ORDER BY id")
        rows = cursor.fetchall()

        assert len(rows) == 2
        assert rows[0][0] == 2
        assert rows[0][1] == "second"
        assert rows[0][2] == pytest.approx(2.5)
        assert rows[1][0] == 3
        assert rows[1][1] == "third"
        assert rows[1][2] == pytest.approx(3.5)
