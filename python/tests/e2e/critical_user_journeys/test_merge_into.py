"""Tests for MERGE INTO (Upsert) operations.

MERGE INTO for upsert operations combining UPDATE and INSERT.
Used by SQLAlchemy's MergeInto construct, Snowfort for replication and OLTP.

Journey 21 - P1
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestMergeInto:
    """Tests for MERGE INTO upsert operations."""

    def test_should_merge_with_update_and_insert(self, execute_query, cursor, tmp_schema):
        """Test MERGE INTO with UPDATE on match and INSERT on no match."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A target table with rows (1, 'original_1', 100) and (2, 'original_2', 200) exists
        target_table = f"{tmp_schema}.merge_target"
        source_table = f"{tmp_schema}.merge_source"

        cursor.execute(f"CREATE TABLE {target_table} (id INT, name VARCHAR, amount INT)")
        cursor.execute(f"INSERT INTO {target_table} VALUES (1, 'original_1', 100), (2, 'original_2', 200)")

        # And A source table with rows (2, 'updated_2', 250) and (3, 'new_3', 300) exists
        cursor.execute(f"CREATE TABLE {source_table} (id INT, name VARCHAR, amount INT)")
        cursor.execute(f"INSERT INTO {source_table} VALUES (2, 'updated_2', 250), (3, 'new_3', 300)")

        # When MERGE INTO target USING source is executed with UPDATE on match and INSERT on no match
        merge_sql = (
            f"MERGE INTO {target_table} AS t "
            f"USING {source_table} AS s "
            "ON t.id = s.id "
            "WHEN MATCHED THEN "
            "UPDATE SET t.name = s.name, t.amount = s.amount "
            "WHEN NOT MATCHED THEN "
            "INSERT (id, name, amount) VALUES (s.id, s.name, s.amount)"
        )
        cursor.execute(merge_sql)

        # Then Merge rowcount should be 2
        assert cursor.rowcount == 2, f"Expected rowcount 2, got {cursor.rowcount}"

        # And Row id=1 should be untouched as (1, 'original_1', 100)
        cursor.execute(f"SELECT * FROM {target_table} WHERE id = 1")
        assert cursor.fetchone() == (1, "original_1", 100)

        # And Row id=2 should be updated to (2, 'updated_2', 250)
        cursor.execute(f"SELECT * FROM {target_table} WHERE id = 2")
        assert cursor.fetchone() == (2, "updated_2", 250)

        # And Row id=3 should be inserted as (3, 'new_3', 300)
        cursor.execute(f"SELECT * FROM {target_table} WHERE id = 3")
        assert cursor.fetchone() == (3, "new_3", 300)
