"""
Integration tests for PEP 249 Cursor objects.
"""

from decimal import Decimal

import pytest

from snowflake.connector.cursor import QueryResultStats, SnowflakeCursor
from snowflake.connector.errors import NotSupportedError, ProgrammingError
from tests.e2e.types.utils import assert_sequential_values


class TestCursorSfqid:
    """Integration tests for Cursor.sfqid property."""

    def test_sfqid_is_none_before_execute(self, connection):
        """Test that sfqid returns None before any query is executed."""
        # Given a new cursor
        cursor = connection.cursor()

        # When accessing sfqid before execute
        result = cursor.sfqid

        # Then it should be None
        assert result is None

    def test_sfqid_returns_valid_uuid_after_execute(self, cursor):
        """Test that sfqid returns a valid query ID after execute."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1")

        # When accessing sfqid
        result = cursor.sfqid

        # Then it should return a valid UUID-like query ID
        assert result is not None

    def test_sfqid_changes_with_each_query(self, cursor):
        """Test that sfqid changes with each executed query."""
        # Given a cursor that executes multiple queries
        cursor.execute("SELECT 1")
        first_sfqid = cursor.sfqid

        cursor.execute("SELECT 2")
        second_sfqid = cursor.sfqid

        cursor.execute("SELECT 3")
        third_sfqid = cursor.sfqid

        # Then each query should have a different sfqid
        assert first_sfqid is not None
        assert second_sfqid is not None
        assert third_sfqid is not None
        assert first_sfqid != second_sfqid
        assert second_sfqid != third_sfqid
        assert first_sfqid != third_sfqid

    def test_sfqid_persists_after_fetchall(self, cursor):
        """Test that sfqid remains accessible after fetching all results."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1, 2, 3")
        sfqid_before = cursor.sfqid

        # When fetching all results
        cursor.fetchall()

        # Then sfqid should still be the same
        assert cursor.sfqid == sfqid_before

    def test_sfqid_persists_after_fetchone(self, cursor):
        """Test that sfqid remains accessible after fetching one result."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1")
        sfqid_before = cursor.sfqid

        # When fetching one result
        cursor.fetchone()

        # Then sfqid should still be the same
        assert cursor.sfqid == sfqid_before


class TestCursorQuery:
    """Integration tests for Cursor.query property."""

    def test_query_is_none_before_execute(self, connection):
        """Test that query returns None before any query is executed."""
        # Given a new cursor
        cursor = connection.cursor()

        # When accessing query before execute
        result = cursor.query

        # Then it should be None
        assert result is None

    def test_query_returns_sql_text_after_execute(self, cursor):
        """Test that query returns the SQL text after execute."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1")

        # When accessing query
        result = cursor.query

        # Then it should return a non-empty string
        assert result is not None
        assert isinstance(result, str)
        assert len(result) > 0

    def test_query_contains_executed_sql(self, cursor):
        """Test that query contains the SQL that was executed."""
        # Given a specific SQL statement
        sql = "SELECT 42 AS answer"

        # When executing the SQL
        cursor.execute(sql)

        # Then query should contain the executed SQL
        assert cursor.query is not None
        assert "42" in cursor.query

    def test_query_changes_with_each_query(self, cursor):
        """Test that query changes with each executed query."""
        # Given a cursor that executes multiple queries
        cursor.execute("SELECT 1")
        first_query = cursor.query

        cursor.execute("SELECT 'hello'")
        second_query = cursor.query

        cursor.execute("SELECT 1, 2, 3")
        third_query = cursor.query

        # Then each query should have a different query text
        assert first_query is not None
        assert second_query is not None
        assert third_query is not None
        assert first_query != second_query
        assert second_query != third_query

    def test_query_persists_after_fetchall(self, cursor):
        """Test that query remains accessible after fetching all results."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1, 2, 3")
        query_before = cursor.query

        # When fetching all results
        cursor.fetchall()

        # Then query should still be the same
        assert cursor.query == query_before

    def test_query_persists_after_fetchone(self, cursor):
        """Test that query remains accessible after fetching one result."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1")
        query_before = cursor.query

        # When fetching one result
        cursor.fetchone()

        # Then query should still be the same
        assert cursor.query == query_before

    def test_query_with_ddl_statement(self, cursor, tmp_schema):
        """Test that query works with DDL statements."""
        # Given a DDL statement
        sql = f"CREATE TABLE {tmp_schema}.test_query_ddl (id INTEGER)"

        # When executing the DDL
        cursor.execute(sql)

        # Then query should return the DDL text
        assert cursor.query is not None
        assert "CREATE TABLE" in cursor.query.upper()

    def test_query_with_dml_statement(self, cursor, tmp_schema):
        """Test that query works with DML statements."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_query_dml (id INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_query_dml VALUES (1)")

        # When accessing query after the INSERT
        result = cursor.query

        # Then it should reflect the INSERT statement
        assert result is not None
        assert "INSERT" in result.upper()

    def test_query_updates_after_new_execute(self, cursor):
        """Test that query updates to reflect the most recent execute."""
        # Given a cursor that executes a SELECT
        cursor.execute("SELECT 1 AS first")
        first_query = cursor.query

        # When executing a different query
        cursor.execute("SELECT 2 AS second")

        # Then query should reflect the new SQL
        assert cursor.query is not None
        assert cursor.query != first_query

    def test_query_with_multiline_sql(self, cursor):
        """Test that query works with multiline SQL statements."""
        # Given a multiline SQL statement
        sql = """
            SELECT
                1 AS col1,
                2 AS col2,
                3 AS col3
        """

        # When executing the multiline SQL
        cursor.execute(sql)

        # Then query should return the SQL text
        assert cursor.query is not None
        assert isinstance(cursor.query, str)
        assert len(cursor.query) > 0


class TestCursorDescription:
    """Integration tests for Cursor.description property."""

    def test_description_is_none_before_execute(self, connection):
        """Test that description returns None before any query is executed."""
        # Given a new cursor
        cursor = connection.cursor()

        # When accessing description before execute
        result = cursor.description

        # Then it should be None
        assert result is None

    def test_description_has_correct_structure(self, cursor):
        """Test that description returns a sequence of 7-item tuples."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1 AS col1")

        # When accessing description
        result = cursor.description

        # Then it should be a list of tuples with 7 elements each
        assert result is not None
        assert len(result) == 1
        assert len(result[0]) == 7

    def test_description_column_name(self, cursor):
        """Test that description contains correct column names."""
        # Given a cursor that executes a query with named columns
        cursor.execute("SELECT 1 AS my_column, 'hello' AS another_column")

        # When accessing description
        result = cursor.description

        # Then column names should match (Snowflake uppercases column names)
        assert result is not None
        assert len(result) == 2
        assert result[0][0] == "MY_COLUMN"
        assert result[1][0] == "ANOTHER_COLUMN"

    def test_description_integer_type(self, cursor):
        """Test description for integer column."""
        # Given a cursor that executes a query returning an integer
        cursor.execute("SELECT 42::INTEGER AS int_col")

        # When accessing description
        result = cursor.description

        # Then type_code should indicate FIXED/NUMBER type (0)
        assert result is not None
        assert result[0][0] == "INT_COL"
        assert result[0][1] == 0  # FIXED type code

    def test_description_string_type(self, cursor):
        """Test description for string column."""
        # Given a cursor that executes a query returning a string
        cursor.execute("SELECT 'hello'::VARCHAR AS str_col")

        # When accessing description
        result = cursor.description

        # Then type_code should indicate TEXT type (2)
        assert result is not None
        assert result[0][0] == "STR_COL"
        assert result[0][1] == 2  # TEXT type code

    def test_description_float_type(self, cursor):
        """Test description for float column."""
        # Given a cursor that executes a query returning a float
        cursor.execute("SELECT 3.14::FLOAT AS float_col")

        # When accessing description
        result = cursor.description

        # Then type_code should indicate REAL type (1)
        assert result is not None
        assert result[0][0] == "FLOAT_COL"
        assert result[0][1] == 1  # REAL type code

    def test_description_boolean_type(self, cursor):
        """Test description for boolean column."""
        # Given a cursor that executes a query returning a boolean
        cursor.execute("SELECT TRUE::BOOLEAN AS bool_col")

        # When accessing description
        result = cursor.description

        # Then type_code should indicate BOOLEAN type (13)
        assert result is not None
        assert result[0][0] == "BOOL_COL"
        assert result[0][1] == 13  # BOOLEAN type code

    def test_description_date_type(self, cursor):
        """Test description for date column."""
        # Given a cursor that executes a query returning a date
        cursor.execute("SELECT '2024-01-15'::DATE AS date_col")

        # When accessing description
        result = cursor.description

        # Then type_code should indicate DATE type (3)
        assert result is not None
        assert result[0][0] == "DATE_COL"
        assert result[0][1] == 3  # DATE type code

    def test_description_timestamp_ntz_type(self, cursor):
        """Test description for timestamp_ntz column."""
        # Given a cursor that executes a query returning a timestamp_ntz
        cursor.execute("SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ AS ts_col")

        # When accessing description
        result = cursor.description

        # Then type_code should indicate TIMESTAMP_NTZ type (8)
        assert result is not None
        assert result[0][0] == "TS_COL"
        assert result[0][1] == 8  # TIMESTAMP_NTZ type code

    def test_description_multiple_columns(self, cursor):
        """Test description with multiple columns of different types."""
        # Given a cursor that executes a query with multiple columns
        cursor.execute("""
            SELECT
                1::INTEGER AS int_col,
                'hello'::VARCHAR AS str_col,
                3.14::FLOAT AS float_col,
                TRUE::BOOLEAN AS bool_col
        """)

        # When accessing description
        result = cursor.description

        # Then all columns should be present with correct types
        assert result is not None
        assert len(result) == 4
        assert result[0][0] == "INT_COL"
        assert result[0][1] == 0  # FIXED
        assert result[1][0] == "STR_COL"
        assert result[1][1] == 2  # TEXT
        assert result[2][0] == "FLOAT_COL"
        assert result[2][1] == 1  # REAL
        assert result[3][0] == "BOOL_COL"
        assert result[3][1] == 13  # BOOLEAN

    def test_description_persists_after_fetchone(self, cursor):
        """Test that description remains accessible after fetching one result."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1 AS col1, 2 AS col2")
        description_before = cursor.description

        # When fetching one result
        cursor.fetchone()

        # Then description should still be the same
        assert cursor.description == description_before

    def test_description_persists_after_fetchall(self, cursor):
        """Test that description remains accessible after fetching all results."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1 AS col1, 2 AS col2")
        description_before = cursor.description

        # When fetching all results
        cursor.fetchall()

        # Then description should still be the same
        assert cursor.description == description_before

    def test_description_updates_with_new_query(self, cursor):
        """Test that description updates when a new query is executed."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1 AS first_col")
        first_description = cursor.description

        # When executing a different query
        cursor.execute("SELECT 'a' AS second_col, 'b' AS third_col")

        # Then description should be updated
        assert cursor.description is not None
        assert cursor.description != first_description
        assert len(cursor.description) == 2
        assert cursor.description[0][0] == "SECOND_COL"
        assert cursor.description[1][0] == "THIRD_COL"

    def test_description_numeric_precision_and_scale(self, cursor):
        """Test that description includes precision and scale for numeric types."""
        # Given a cursor that executes a query with a decimal column
        cursor.execute("SELECT 123.45::NUMBER(10, 2) AS decimal_col")

        # When accessing description
        result = cursor.description

        # Then precision (index 4) and scale (index 5) should be populated
        assert result is not None
        assert result[0][0] == "DECIMAL_COL"
        # precision is at index 4, scale is at index 5
        assert result[0][4] is not None
        assert result[0][5] is not None
        # These may be None or have values depending on server response
        # At minimum, verify the structure is correct
        assert len(result[0]) == 7


class TestCursorRowcount:
    """Integration tests for Cursor.rowcount property."""

    def test_rowcount_is_none_before_execute(self, connection):
        """Test that rowcount returns None before any query is executed."""
        # Given a new cursor
        cursor = connection.cursor()

        # When accessing rowcount before execute
        result = cursor.rowcount

        # Then it should be None (per PEP 249)
        assert result is None

    def test_rowcount_after_select_single_row(self, cursor):
        """Test rowcount after a SELECT query returning single row."""
        # Given a cursor that executes a SELECT query
        cursor.execute("SELECT 1")

        # When accessing rowcount
        result = cursor.rowcount

        # Then rowcount should be 1
        assert isinstance(result, int)
        assert result == 1

    def test_rowcount_after_select_multiple_rows(self, cursor):
        """Test rowcount after a SELECT query returning multiple rows."""
        # Given a cursor that executes a SELECT query returning 5 rows
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 5))")

        # When accessing rowcount
        result = cursor.rowcount

        # Then rowcount should reflect the number of rows
        assert isinstance(result, int)
        assert result == 5

    def test_rowcount_after_insert(self, cursor, tmp_schema):
        """Test rowcount after INSERT statement."""
        # Given a table to insert into
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_rowcount (id INTEGER, name VARCHAR)")

        # When inserting a single row
        cursor.execute(f"INSERT INTO {tmp_schema}.test_rowcount VALUES (1, 'test')")

        # Then rowcount should be 1
        assert cursor.rowcount == 1

    def test_rowcount_after_multi_row_select(self, cursor, tmp_schema):
        """Test rowcount after selecting multiple rows."""
        # When selecting multiple rows
        cursor.execute("""SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 5))""")

        # Then rowcount should be 5
        assert cursor.rowcount == 5

    def test_rowcount_after_update(self, cursor, tmp_schema):
        """Test rowcount after UPDATE statement."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_update (id INTEGER, value INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_update VALUES (1, 10), (2, 20), (3, 30)")

        # When updating some rows
        cursor.execute(f"UPDATE {tmp_schema}.test_update SET value = 100 WHERE id <= 2")

        # Then rowcount should return 2
        assert cursor.rowcount == 2

    def test_rowcount_after_delete(self, cursor, tmp_schema):
        """Test rowcount after DELETE statement."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_delete (id INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_delete VALUES (1), (2), (3), (4), (5)")

        # When deleting some rows
        cursor.execute(f"DELETE FROM {tmp_schema}.test_delete WHERE id > 2")

        # Then rowcount should reflect only affected rows
        assert cursor.rowcount == 3

    def test_rowcount_persists_after_fetch(self, cursor):
        """Test that rowcount persists after fetching results."""
        # Given a cursor that executes a SELECT query
        cursor.execute("SELECT 1, 2, 3")
        rowcount_before_fetch = cursor.rowcount

        # When fetching results
        cursor.fetchall()

        # Then rowcount should persist after fetch
        assert cursor.rowcount == rowcount_before_fetch

    def test_rowcount_updates_with_new_query(self, cursor):
        """Test that rowcount updates when a new query is executed."""
        # Given a cursor that executes a SELECT returning 1 row
        cursor.execute("SELECT 1")
        first_rowcount = cursor.rowcount
        assert first_rowcount == 1

        # When executing a SELECT returning multiple rows
        cursor.execute("SELECT * FROM (SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3)")

        # Then rowcount should be updated to 3
        assert cursor.rowcount == 3
        assert cursor.rowcount != first_rowcount

    def test_rowcount_after_delete_zero_rows(self, cursor, tmp_schema):
        """Test rowcount after DELETE statement that affects 0 rows."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_delete_zero (id INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_delete_zero VALUES (1), (2), (3)")

        # When deleting with a condition that matches no rows
        cursor.execute(f"DELETE FROM {tmp_schema}.test_delete_zero WHERE id > 100")

        # Then rowcount should be 0
        assert cursor.rowcount == 0

    def test_rowcount_after_update_zero_rows(self, cursor, tmp_schema):
        """Test rowcount after UPDATE statement that affects 0 rows."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_update_zero (id INTEGER, value INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_update_zero VALUES (1, 10), (2, 20), (3, 30)")

        # When updating with a condition that matches no rows
        cursor.execute(f"UPDATE {tmp_schema}.test_update_zero SET value = 999 WHERE id > 100")

        # Then rowcount should be 0
        assert cursor.rowcount == 0

    def test_rowcount_after_ddl(self, cursor, tmp_schema):
        """Test rowcount after DDL statement."""
        # When executing a DDL statement
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_ddl_rowcount (id INTEGER)")

        # Then rowcount should be 1
        assert cursor.rowcount == 1


class TestCursorStats:
    """Integration tests for Cursor.stats property."""

    def test_stats_returns_all_none_before_execute(self, connection):
        """Test that stats returns all-None QueryResultStats before any query is executed."""
        # Given a new cursor
        cursor = connection.cursor()

        # When accessing stats before execute
        result = cursor.stats

        # Then all fields should be None
        assert isinstance(result, QueryResultStats)
        assert result == QueryResultStats(None, None, None, None)

    def test_stats_returns_query_result_stats_type(self, cursor):
        """Test that stats always returns a QueryResultStats instance."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 1")

        # When accessing stats
        result = cursor.stats

        # Then it should be a QueryResultStats instance
        assert isinstance(result, QueryResultStats)

    def test_stats_after_insert(self, cursor, tmp_schema):
        """Test stats.num_rows_inserted is populated after INSERT."""
        # Given a table to insert into
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_insert (id INTEGER, name VARCHAR)")

        # When inserting rows
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_insert VALUES (1, 'a'), (2, 'b'), (3, 'c')")

        # Then num_rows_inserted should reflect the number of inserted rows
        assert cursor.stats.num_rows_inserted == 3

    def test_stats_after_single_row_insert(self, cursor, tmp_schema):
        """Test stats.num_rows_inserted for a single row INSERT."""
        # Given a table
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_single_insert (id INTEGER)")

        # When inserting a single row
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_single_insert VALUES (1)")

        # Then num_rows_inserted should be 1
        assert cursor.stats.num_rows_inserted == 1

    def test_stats_after_update(self, cursor, tmp_schema):
        """Test stats.num_rows_updated is populated after UPDATE."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_update (id INTEGER, value INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_update VALUES (1, 10), (2, 20), (3, 30)")

        # When updating some rows
        cursor.execute(f"UPDATE {tmp_schema}.test_stats_update SET value = 100 WHERE id <= 2")

        # Then num_rows_updated should reflect the number of updated rows
        assert cursor.stats.num_rows_updated == 2

    def test_stats_after_update_zero_rows(self, cursor, tmp_schema):
        """Test stats when no rows match the UPDATE condition."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_update_zero (id INTEGER, value INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_update_zero VALUES (1, 10), (2, 20)")

        # When updating with a condition that matches no rows
        cursor.execute(f"UPDATE {tmp_schema}.test_stats_update_zero SET value = 999 WHERE id > 100")

        # Then stats should have no DML counts (server omits stats when 0 rows affected)
        assert cursor.stats.num_rows_updated is None

    def test_stats_after_delete(self, cursor, tmp_schema):
        """Test stats.num_rows_deleted is populated after DELETE."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_delete (id INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_delete VALUES (1), (2), (3), (4), (5)")

        # When deleting some rows
        cursor.execute(f"DELETE FROM {tmp_schema}.test_stats_delete WHERE id > 2")

        # Then num_rows_deleted should reflect the number of deleted rows
        assert cursor.stats.num_rows_deleted == 3

    def test_stats_after_delete_zero_rows(self, cursor, tmp_schema):
        """Test stats when no rows match the DELETE condition."""
        # Given a table with data
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_delete_zero (id INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_delete_zero VALUES (1), (2), (3)")

        # When deleting with a condition that matches no rows
        cursor.execute(f"DELETE FROM {tmp_schema}.test_stats_delete_zero WHERE id > 100")

        # Then stats should have no DML counts (server omits stats when 0 rows affected)
        assert cursor.stats.num_rows_deleted is None

    def test_stats_after_select(self, cursor):
        """Test that stats has no DML counts after a SELECT statement."""
        # Given a cursor that executes a SELECT query
        cursor.execute("SELECT 1")

        # When accessing stats
        result = cursor.stats

        # Then DML-specific fields should be None (SELECT is not a DML operation)
        assert result.num_rows_inserted is None
        assert result.num_rows_deleted is None
        assert result.num_rows_updated is None

    def test_stats_persists_after_fetchall(self, cursor, tmp_schema):
        """Test that stats persists after fetching results."""
        # Given a table with an INSERT
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_persist (id INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_persist VALUES (1), (2)")
        stats_before = cursor.stats

        # When doing a SELECT and fetching all
        cursor.execute(f"SELECT * FROM {tmp_schema}.test_stats_persist")
        cursor.fetchall()

        # Then stats should reflect the latest query (the SELECT)
        # and the old stats from the INSERT should no longer be there
        assert cursor.stats != stats_before or stats_before.num_rows_inserted is None

    def test_stats_updates_with_new_execute(self, cursor, tmp_schema):
        """Test that stats updates to reflect the most recent DML operation."""
        # Given a table
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_updates (id INTEGER, value INTEGER)")

        # When performing an INSERT
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_updates VALUES (1, 10), (2, 20)")
        insert_stats = cursor.stats

        # Then INSERT stats should be populated
        assert insert_stats.num_rows_inserted == 2

        # When performing a DELETE
        cursor.execute(f"DELETE FROM {tmp_schema}.test_stats_updates WHERE id = 1")
        delete_stats = cursor.stats

        # Then DELETE stats should be populated
        assert delete_stats.num_rows_deleted == 1

    def test_stats_after_insert_select(self, cursor, tmp_schema):
        """Test stats after INSERT ... SELECT."""
        # Given source and target tables
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_src (id INTEGER)")
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_src VALUES (1), (2), (3)")
        cursor.execute(f"CREATE TABLE {tmp_schema}.test_stats_dst (id INTEGER)")

        # When inserting via SELECT
        cursor.execute(f"INSERT INTO {tmp_schema}.test_stats_dst SELECT * FROM {tmp_schema}.test_stats_src")

        # Then num_rows_inserted should reflect the number of rows inserted
        assert cursor.stats.num_rows_inserted == 3

    def test_stats_num_dml_duplicates_after_update_with_duplicate_join(self, cursor):
        """Test stats.num_dml_duplicates is populated when UPDATE joins to duplicate source rows."""
        # Given a target with one row and a source where multiple rows match the same target
        cursor.execute("CREATE OR REPLACE TEMP TABLE test_dup_src (c1 INT, c2 INT)")
        cursor.execute("CREATE OR REPLACE TEMP TABLE test_dup_target (c INT)")
        cursor.execute("INSERT INTO test_dup_src VALUES (0, 100), (0, 200), (0, 300)")
        cursor.execute("INSERT INTO test_dup_target VALUES (0)")

        # When updating via a join that produces duplicate matches
        cursor.execute("""
            UPDATE test_dup_target t
            SET c = s.c1
            FROM test_dup_src s
            WHERE s.c1 = t.c
        """)

        # Then one source row wins the update and the rest are counted as duplicates
        assert cursor.stats.num_rows_updated == 1
        assert cursor.stats.num_dml_duplicates == 1


class TestCursorRownumber:
    """Integration tests for Cursor.rownumber property."""

    def test_rownumber_is_none_before_fetch(self, cursor):
        """Test that rownumber is None after execute but before any fetch."""
        # Given a cursor that executes a query
        cursor.execute("SELECT * FROM VALUES (1), (2)")

        # When accessing rownumber before fetching
        result = cursor.rownumber

        # Then it should be None (no rows fetched yet)
        assert result is None

    def test_rownumber_starts_at_zero_after_first_fetch(self, cursor):
        """Test that rownumber is 0 after the first fetchone."""
        # Given a cursor that executes a query
        cursor.execute("SELECT * FROM VALUES (1), (2)")

        # When fetching the first row
        cursor.fetchone()

        # Then rownumber should be 0 (0-based index)
        assert cursor.rownumber == 0

    def test_rownumber_increments_with_fetchone(self, cursor):
        """Test that rownumber increments with each fetchone call."""
        # Given a cursor that executes a query with multiple rows
        cursor.execute("SELECT * FROM VALUES (1), (2), (3)")

        # When fetching rows one by one
        cursor.fetchone()
        assert cursor.rownumber == 0

        cursor.fetchone()
        assert cursor.rownumber == 1

        cursor.fetchone()
        assert cursor.rownumber == 2

    def test_rownumber_does_not_increment_past_end(self, cursor):
        """Test that rownumber does not increment when fetchone returns None."""
        # Given a cursor with a single row
        cursor.execute("SELECT 1")
        cursor.fetchone()
        assert cursor.rownumber == 0

        # When fetching past the end
        cursor.fetchone()

        # Then rownumber should stay at last value
        assert cursor.rownumber == 0

    def test_rownumber_resets_with_new_execute(self, cursor):
        """Test that rownumber resets when a new query is executed."""
        # Given a cursor that has fetched rows
        cursor.execute("SELECT * FROM VALUES (1), (2), (3)")
        cursor.fetchone()
        cursor.fetchone()
        assert cursor.rownumber == 1

        # When executing a new query
        cursor.execute("SELECT * FROM VALUES (10), (20)")

        # Then rownumber should be reset to None (no rows fetched yet)
        assert cursor.rownumber is None

    def test_rownumber_increments_with_fetchmany(self, cursor):
        """Test that rownumber increments correctly with fetchmany."""
        # Given a cursor with multiple rows
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )

        # When fetching multiple rows at once
        cursor.fetchmany(3)

        # Then rownumber should reflect the last row fetched (0-based)
        assert cursor.rownumber == 2

    def test_rownumber_updated_by_fetchall(self, cursor):
        """Test that rownumber reflects total rows fetched after fetchall."""
        # Given a cursor with multiple rows
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )

        # When fetching all rows at once
        cursor.fetchall()

        # Then rownumber should be the 0-based index of the last row
        assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchall_after_partial_fetchone(self, cursor):
        """Test that rownumber is correct when fetchall follows partial fetchone."""
        # Given a cursor with multiple rows, partially consumed
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        cursor.fetchone()
        cursor.fetchone()
        assert cursor.rownumber == 1

        # When fetching remaining rows
        cursor.fetchall()

        # Then rownumber should be the 0-based index of the last row
        assert cursor.rownumber == 4

    def test_rownumber_fetchall_on_empty_result(self, cursor):
        """Test that rownumber stays None when fetchall returns no rows."""
        # Given a cursor with an empty result set
        cursor.execute("SELECT 1 WHERE FALSE")

        # When fetching all (no rows)
        cursor.fetchall()

        # Then rownumber should remain None
        assert cursor.rownumber is None

    def test_rownumber_fetchmany_then_fetchall(self, cursor):
        """Test rownumber is correct after fetchmany followed by fetchall."""
        # Given a cursor with multiple rows
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 10))
            ORDER BY 1
            """
        )

        # When fetching some rows with fetchmany, then the rest with fetchall
        cursor.fetchmany(3)
        assert cursor.rownumber == 2

        cursor.fetchall()

        # Then rownumber should be the 0-based index of the last row
        assert cursor.rownumber == 9


class TestCursorSqlstate:
    """Integration tests for Cursor.sqlstate property."""

    def test_sqlstate_is_none_before_execute(self, connection):
        """Test that sqlstate returns None before any query is executed."""
        cursor = connection.cursor()
        assert cursor.sqlstate is None

    def test_sqlstate_none_after_successful_select(self, cursor):
        """Successful queries set sqlstate to None (00000 is normalized)."""
        cursor.execute("SELECT 1")
        assert cursor.sqlstate is None

    def test_sqlstate_none_after_dml(self, cursor):
        """Successful DDL/DML statements set sqlstate to None."""
        cursor.execute("CREATE TEMPORARY TABLE test_sqlstate_dml (id INT)")
        assert cursor.sqlstate is None

        cursor.execute("INSERT INTO test_sqlstate_dml VALUES (1)")
        assert cursor.sqlstate is None

    def test_sqlstate_stays_none_across_executes(self, cursor):
        """sqlstate is refreshed on every execute call."""
        cursor.execute("SELECT 1")
        first = cursor.sqlstate

        cursor.execute("SELECT 2")
        second = cursor.sqlstate

        assert first is None
        assert second is None

    def test_sqlstate_persists_after_fetchall(self, cursor):
        """sqlstate is not cleared by fetching results."""
        cursor.execute("SELECT 1, 2, 3")
        cursor.fetchall()
        assert cursor.sqlstate is None

    def test_sqlstate_persists_after_fetchone(self, cursor):
        """sqlstate is not cleared by fetching results."""
        cursor.execute("SELECT 1")
        cursor.fetchone()
        assert cursor.sqlstate is None

    def test_sqlstate_set_on_failed_execute(self, cursor):
        """sqlstate is captured from the error when a query fails."""
        with pytest.raises(ProgrammingError):
            cursor.execute("SELECT * FROM nonexistent_table_that_does_not_exist_42")

        assert cursor.sqlstate == "42S02"

    def test_sqlstate_transitions_across_success_and_failure(self, cursor):
        """sqlstate updates correctly through None -> error -> None."""
        cursor.execute("SELECT 1")
        assert cursor.sqlstate is None

        with pytest.raises(ProgrammingError):
            cursor.execute("SELECT * FROM nonexistent_table_that_does_not_exist_42")
        assert cursor.sqlstate == "42S02"

        cursor.execute("SELECT 2")
        assert cursor.sqlstate is None


class TestCursorStatementLifecycle:
    """Integration tests for statement handle lifecycle (create/release)."""

    def test_sequential_executes_do_not_leak(self, cursor):
        """Test that many sequential execute calls work without resource exhaustion."""
        # Given a cursor that executes many queries in sequence
        for i in range(50):
            cursor.execute(f"SELECT {i}")
            row = cursor.fetchone()
            assert row[0] == i

    def test_cursor_context_manager_cleanup(self, connection):
        """Test that cursor context manager properly cleans up resources."""
        # Given a cursor used as a context manager
        with connection.cursor() as cur:
            cur.execute("SELECT 1")
            result = cur.fetchone()
            assert result == (1,)

        # When the context manager exits, the cursor should be closed
        assert cur.is_closed()

    def test_cursor_usable_after_close_and_reopen(self, connection):
        """Test that closing a cursor and creating a new one works."""
        # Given a cursor that executes and is then closed
        cursor1 = connection.cursor()
        cursor1.execute("SELECT 1")
        cursor1.fetchone()
        cursor1.close()
        assert cursor1.is_closed()

        # When creating a new cursor
        cursor2 = connection.cursor()
        cursor2.execute("SELECT 2")
        result = cursor2.fetchone()

        # Then the new cursor should work normally
        assert result == (2,)
        cursor2.close()

    def test_execute_after_previous_execute(self, cursor):
        """Test that a second execute properly replaces the first result."""
        # Given a cursor that executes a query
        cursor.execute("SELECT 'first'")
        assert cursor.fetchone()[0] == "first"

        # When executing a second query on the same cursor
        cursor.execute("SELECT 'second'")

        # Then the second result should be available
        assert cursor.fetchone()[0] == "second"

    def test_multiple_cursors_independent(self, connection):
        """Test that multiple cursors operate independently."""
        cursor1 = connection.cursor()
        cursor2 = connection.cursor()

        cursor1.execute("SELECT 1")
        cursor2.execute("SELECT 2")

        assert cursor1.fetchone() == (1,)
        assert cursor2.fetchone() == (2,)

        cursor1.close()
        cursor2.close()

    def test_close_is_idempotent(self, cursor):
        """Test that calling close() multiple times is safe."""
        cursor.execute("SELECT 1")
        cursor.fetchone()

        # First close
        cursor.close()
        assert cursor.is_closed()

        # Second close should not raise an error
        cursor.close()
        assert cursor.is_closed()

    def test_statement_cleanup_on_execute_failure(self, cursor):
        """Test that statement handles are cleaned up even if execute fails."""
        # Execute a successful query first
        cursor.execute("SELECT 1")
        assert cursor.fetchone() == (1,)

        # Try to execute an invalid query
        try:
            cursor.execute("SELECT * FROM nonexistent_table_xyz")
        except Exception:
            pass  # Expected to fail

        # Should be able to execute a new valid query after failure
        cursor.execute("SELECT 2")
        assert cursor.fetchone() == (2,)


class TestCursorMethods:
    """Test Cursor object methods."""

    def test_close_cursor(self, cursor):
        """Test closing a cursor."""
        assert not cursor.is_closed()
        cursor.close()
        assert cursor.is_closed()

    @pytest.mark.skip_reference(
        reason="Reference driver forwards callproc to server instead of raising NotSupportedError"
    )
    def test_callproc_not_implemented(self, cursor):
        """Test that callproc raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.callproc("test_proc", [1, 2, 3])
        assert "callproc is not implemented" in str(excinfo.value)

    def test_executemany_is_callable(self, cursor):
        """Test that executemany is callable (basic smoke test)."""
        # Just verify it's callable and accepts empty sequence without error
        cursor.executemany("INSERT INTO test VALUES (?)", [])

    @pytest.mark.skip_reference(
        reason="Reference driver returns None from nextset instead of raising NotSupportedError"
    )
    def test_nextset_not_implemented(self, cursor):
        """Test that nextset raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.nextset()
        assert "nextset is not implemented" in str(excinfo.value)

    def test_setinputsizes_no_op(self, cursor):
        """Test that setinputsizes is a no-op."""
        # Should not raise any exception
        cursor.setinputsizes([10, 20, 30])

    def test_setoutputsize_no_op(self, cursor):
        """Test that setoutputsize is a no-op."""
        # Should not raise any exception
        cursor.setoutputsize(100)
        cursor.setoutputsize(100, 1)


class TestCursorContextManager:
    """Test Cursor context manager functionality."""

    def test_context_manager_entry(self, cursor):
        """Test entering cursor context manager."""
        with cursor as c:
            assert c is cursor

    def test_context_manager_exit(self, cursor):
        """Test exiting cursor context manager."""
        with cursor:
            pass

        assert cursor.is_closed()

    def test_context_manager_exit_with_exception(self, cursor):
        """Test exiting cursor context manager with exception."""
        try:
            with cursor:
                raise ValueError("Test exception")
        except ValueError:
            pass

        assert cursor.is_closed()


class TestCursorDatabaseQueries:
    """Integration tests for Cursor with real database queries."""

    def test_simple_select(self, cursor):
        """Test simple select."""
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        # Result format may vary between connectors, just check it's not None
        assert result is not None

    @pytest.mark.parametrize("data_size", [1000, 10000])
    def test_large_result(self, cursor, data_size):
        """Test large result."""
        cursor.execute(f"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => {data_size})) v ORDER BY id")
        rows = cursor.fetchall()
        assert len(rows) == data_size

        for i, row in enumerate(rows):
            assert row == (i,)


class TestCursorFetch:
    """Test cursor fetch operations."""

    def test_execute_returns_cursor(self, cursor):
        """Test execute returns cursor"""
        r = cursor.execute("SELECT 1")
        assert isinstance(r, SnowflakeCursor)
        assert r is cursor

    def test_fetchone_single_value(self, cursor):
        """Test fetchone with a single value."""
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        assert result == (1,)

    def test_fetchone_multiple_columns(self, cursor):
        """Test fetchone with multiple columns."""
        cursor.execute("SELECT 1, 'hello', 3.14")
        result = cursor.fetchone()
        assert result == (1, "hello", Decimal("3.14"))

    def test_fetchone_returns_none_when_exhausted(self, cursor):
        """Test fetchone returns None when no more rows."""
        cursor.execute("SELECT 1")
        cursor.fetchone()  # Consume the row
        result = cursor.fetchone()
        assert result is None

    def test_fetchall_single_row(self, cursor):
        """Test fetchall with a single row."""
        cursor.execute("SELECT 42")
        result = cursor.fetchall()
        assert len(result) == 1
        assert result[0] == (42,)

    def test_fetchall_multiple_rows(self, cursor):
        """Test fetchall with multiple rows."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 10))
            ORDER BY 1
        """
        )
        result = cursor.fetchall()
        assert result == [(i,) for i in range(10)]

    def test_fetchall_multiple_columns(self, cursor):
        """Test fetchall with multiple columns."""
        cursor.execute("SELECT 1, 'hello', 3.14")
        result = cursor.fetchall()
        assert result == [(1, "hello", Decimal("3.14"))]

    def test_fetchall_empty_result(self, cursor):
        """Test fetchall with empty result."""
        cursor.execute("SELECT 1 WHERE FALSE")
        result = cursor.fetchall()
        assert result == []

    def test_fetchmany_default_arraysize(self, cursor):
        """Test fetchmany with default arraysize."""
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        cursor.arraysize = 2
        result = cursor.fetchmany()
        assert result == [(0,), (1,)]

    def test_fetchmany_with_explicit_size(self, cursor):
        """Test fetchmany with explicit size argument."""
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 10))
            ORDER BY 1
            """
        )
        result = cursor.fetchmany(3)
        assert result == [(0,), (1,), (2,)]

    def test_fetchmany_returns_fewer_when_exhausted(self, cursor):
        """Test fetchmany returns fewer rows when result set is exhausted."""
        cursor.execute("SELECT 1 UNION ALL SELECT 2")
        result = cursor.fetchmany(10)
        assert len(result) == 2

    def test_fetchmany_returns_empty_after_exhausted(self, cursor):
        """Test fetchmany returns empty list after all rows consumed."""
        cursor.execute("SELECT 1")
        cursor.fetchmany(10)  # Consume all rows
        result = cursor.fetchmany(10)
        assert result == []

    def test_fetchmany_with_size_zero(self, cursor):
        """Test fetchmany(0) returns empty list."""
        cursor.execute("SELECT 1")
        result = cursor.fetchmany(0)
        assert result == []

    def test_fetchmany_negative_size_raises_error(self, cursor):
        """Test fetchmany with negative size raises ProgrammingError."""
        cursor.execute("SELECT 1")
        with pytest.raises(ProgrammingError) as excinfo:
            cursor.fetchmany(-1)
        assert "The number of rows is not zero or positive number" in str(excinfo.value)

    def test_fetchmany_sequential_calls(self, cursor):
        """Test multiple sequential fetchmany calls."""
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 10))
            ORDER BY 1
            """
        )
        first = cursor.fetchmany(3)
        second = cursor.fetchmany(3)
        third = cursor.fetchmany(3)
        fourth = cursor.fetchmany(3)

        assert first == [(0,), (1,), (2,)]
        assert second == [(3,), (4,), (5,)]
        assert third == [(6,), (7,), (8,)]
        assert fourth == [(9,)]

    def test_fetchmany_mixed_with_fetchone(self, cursor):
        """Test fetchmany mixed with fetchone."""
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        first = cursor.fetchone()
        batch = cursor.fetchmany(2)
        last = cursor.fetchone()

        assert first == (0,)
        assert batch == [(1,), (2,)]
        assert last == (3,)

    def test_fetchmany_mixed_with_fetchall(self, cursor):
        """Test fetchmany followed by fetchall."""
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        batch = cursor.fetchmany(2)
        remaining = cursor.fetchall()

        assert batch == [(0,), (1,)]
        assert remaining == [(2,), (3,), (4,)]

    def test_fetchmany_with_multiple_columns(self, cursor):
        """Test fetchmany with multiple columns."""
        cursor.execute("SELECT 1, 'hello', 3.14 UNION ALL SELECT 2, 'world', 2.71")
        result = cursor.fetchmany(2)
        assert len(result) == 2
        assert result[0] == (1, "hello", Decimal("3.14"))
        assert result[1] == (2, "world", Decimal("2.71"))


class TestCursorIteration:
    """Test cursor iteration."""

    def test_cursor_is_iterable(self, cursor):
        """Test cursor can be iterated."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
        """
        )
        rows = list(cursor)
        assert rows == [(i,) for i in range(5)]

    def test_cursor_iteration_order(self, cursor):
        """Test cursor iteration maintains order."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 100))
            ORDER BY n DESC
        """
        )
        rows = list(cursor)
        for i, row in enumerate(rows):
            assert row == (99 - i,), f"Expected ({99 - i},), got {row}"

    def test_mixed_fetchone_and_iteration(self, cursor):
        """Test mixing fetchone and iteration."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
        """
        )
        # Fetch first row
        first = cursor.fetchone()
        assert first == (0,)
        # Iterate rest
        remaining = list(cursor)
        assert remaining == [(1,), (2,), (3,), (4,)]


class TestCursorLargeResults:
    """Test cursor with large result sets spanning multiple batches."""

    N_ROWS = 5_000

    @pytest.mark.parametrize("data_size", [N_ROWS, 20_000])
    def test_large_result_fetchall(self, cursor, data_size):
        """Test fetchall with large results."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            f"""
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => {data_size}))
            ORDER BY 1
        """
        )
        result = cursor.fetchall()
        values = [row[0] for row in result]
        assert_sequential_values(values, data_size)

    def test_large_result_iteration(self, cursor):
        """Test iteration over large results."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            f"""
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))
            ORDER BY 1
        """
        )
        rows = list(cursor)
        values = [row[0] for row in rows]
        assert_sequential_values(values, self.N_ROWS)

    def test_large_result_with_multiple_columns(self, cursor):
        """Test large result with multiple columns."""
        # Use ROW_NUMBER() in a CTE to get consecutive integers starting from 0.
        # seq4() doesn't guarantee consecutive values in parallel execution,
        # and window functions need to be computed once then reused.
        cursor.execute(
            f"""
            WITH base AS (
                SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as id
                FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))
            )
            SELECT id, id * 2 as doubled, id % 10 as mod10 FROM base
            ORDER BY 1
        """
        )
        result = cursor.fetchall()
        assert_sequential_values(
            result,
            self.N_ROWS,
            transform=lambda i: (i, i * 2, i % 10),
        )

    def test_partial_batch_consumption(self, cursor):
        """Test partial consumption of batches."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            f"""
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))
            ORDER BY 1
        """
        )
        # Fetch only some rows
        for _ in range(self.N_ROWS // 10):
            cursor.fetchone()
        # Fetch remaining
        remaining = cursor.fetchall()
        values = [row[0] for row in remaining]
        assert_sequential_values(values, self.N_ROWS - self.N_ROWS // 10, start=self.N_ROWS // 10)


class TestCursorMultipleQueries:
    """Test cursor with multiple queries."""

    def test_sequential_queries(self, cursor):
        """Test sequential queries on same cursor."""
        cursor.execute("SELECT 1")
        result1 = cursor.fetchone()
        assert result1 == (1,)

        cursor.execute("SELECT 2, 3")
        result2 = cursor.fetchone()
        assert result2 == (2, 3)

    def test_new_query_resets_iterator(self, cursor):
        """Test new query resets the iterator state."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 100))")
        # Partially consume
        for _ in range(10):
            cursor.fetchone()

        # New query should reset
        cursor.execute("SELECT 42")
        result = cursor.fetchone()
        assert result == (42,)

    def test_fetchall_after_partial_fetch(self, cursor):
        """Test fetchall after partial fetchone calls."""
        # Use ROW_NUMBER() for consecutive integers; seq4() may skip values in parallel.
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 10))
            ORDER BY n
        """
        )
        # Fetch first 3
        r1 = cursor.fetchone()
        r2 = cursor.fetchone()
        r3 = cursor.fetchone()
        assert r1 == (0,)
        assert r2 == (1,)
        assert r3 == (2,)

        # Fetch remaining
        remaining = cursor.fetchall()
        assert remaining == [(i,) for i in range(3, 10)]

    def test_fetchone_fetchmany_fetchall_sequence(self, cursor):
        """Test fetchone, fetchmany, and fetchall in sequence on same result set."""
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 20))
            ORDER BY n
            """
        )
        # First fetchone
        row1 = cursor.fetchone()
        assert row1 == (0,)

        # Then fetchmany
        batch = cursor.fetchmany(5)
        assert batch == [(i,) for i in range(1, 6)]

        # Finally fetchall gets the remainder
        remainder = cursor.fetchall()
        assert remainder == [(i,) for i in range(6, 20)]

    def test_fetchmany_then_execute_resets_and_fetchmany_again(self, cursor):
        """Test that second execute resets state and fetchmany starts anew."""
        # First query
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 15))
            ORDER BY n
            """
        )
        # Fetch some rows
        batch1 = cursor.fetchmany(5)
        assert batch1 == [(i,) for i in range(5)]

        # Second execute should reset state
        cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) + 100 as n
            FROM TABLE(GENERATOR(ROWCOUNT => 10))
            ORDER BY n
            """
        )
        # fetchmany should start anew from the new result set
        batch2 = cursor.fetchmany(4)
        assert batch2 == [(101,), (102,), (103,), (104,)]

        # Continue fetching from new result set
        batch3 = cursor.fetchmany(3)
        assert batch3 == [(105,), (106,), (107,)]


class TestDictCursorCreation:
    """Test DictCursor creation via connection.cursor()."""

    def test_create_dict_cursor(self, connection):
        """Test that DictCursor can be created via connection.cursor()."""
        from snowflake.connector.cursor import DictCursor

        with connection.cursor(DictCursor) as cur:
            assert isinstance(cur, DictCursor)

    def test_dict_cursor_is_base_cursor_subclass(self):
        """Test that DictCursor is a subclass of BaseCursor."""
        from snowflake.connector.cursor import DictCursor, SnowflakeCursorBase

        assert issubclass(DictCursor, SnowflakeCursorBase)


class TestDictCursorFetchOne:
    """Test DictCursor fetchone operations."""

    def test_fetchone_returns_dict(self, dict_cursor):
        """Test fetchone returns a dictionary with column names as keys."""
        dict_cursor.execute("SELECT 1 AS id, 'hello' AS name")
        result = dict_cursor.fetchone()
        assert isinstance(result, dict)
        assert result == {"ID": 1, "NAME": "hello"}

    def test_fetchone_multiple_columns(self, dict_cursor):
        """Test fetchone with multiple columns."""
        dict_cursor.execute("SELECT 1 AS a, 'hello' AS b, 3.14 AS c")
        result = dict_cursor.fetchone()
        assert isinstance(result, dict)
        assert result["A"] == 1
        assert result["B"] == "hello"
        assert result["C"] == Decimal("3.14")

    def test_fetchone_returns_none_when_exhausted(self, dict_cursor):
        """Test fetchone returns None when no more rows."""
        dict_cursor.execute("SELECT 1 AS id")
        dict_cursor.fetchone()
        result = dict_cursor.fetchone()
        assert result is None

    def test_fetchone_sequential_rows(self, dict_cursor):
        """Test fetchone returns rows sequentially as dicts."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 3))
            ORDER BY 1
            """
        )
        r1 = dict_cursor.fetchone()
        r2 = dict_cursor.fetchone()
        r3 = dict_cursor.fetchone()
        assert r1 == {"N": 0}
        assert r2 == {"N": 1}
        assert r3 == {"N": 2}


class TestDictCursorFetchMany:
    """Test DictCursor fetchmany operations."""

    def test_fetchmany_returns_list_of_dicts(self, dict_cursor):
        """Test fetchmany returns a list of dictionaries."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        result = dict_cursor.fetchmany(3)
        assert len(result) == 3
        assert all(isinstance(row, dict) for row in result)
        assert result == [{"N": 0}, {"N": 1}, {"N": 2}]

    def test_fetchmany_default_arraysize(self, dict_cursor):
        """Test fetchmany with default arraysize."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        dict_cursor.arraysize = 2
        result = dict_cursor.fetchmany()
        assert result == [{"N": 0}, {"N": 1}]

    def test_fetchmany_returns_empty_after_exhausted(self, dict_cursor):
        """Test fetchmany returns empty list after all rows consumed."""
        dict_cursor.execute("SELECT 1 AS id")
        dict_cursor.fetchmany(10)
        result = dict_cursor.fetchmany(10)
        assert result == []


class TestDictCursorFetchAll:
    """Test DictCursor fetchall operations."""

    def test_fetchall_returns_list_of_dicts(self, dict_cursor):
        """Test fetchall returns a list of dictionaries."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        result = dict_cursor.fetchall()
        assert len(result) == 5
        assert all(isinstance(row, dict) for row in result)
        assert result == [{"N": i} for i in range(5)]

    def test_fetchall_multiple_columns(self, dict_cursor):
        """Test fetchall with multiple columns returns dicts."""
        dict_cursor.execute("SELECT 1 AS a, 'hello' AS b UNION ALL SELECT 2, 'world'")
        result = dict_cursor.fetchall()
        assert len(result) == 2
        assert all(isinstance(row, dict) for row in result)
        assert result[0]["A"] == 1
        assert result[0]["B"] == "hello"
        assert result[1]["A"] == 2
        assert result[1]["B"] == "world"

    def test_fetchall_after_partial_fetchone(self, dict_cursor):
        """Test fetchall after partial fetchone calls."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        first = dict_cursor.fetchone()
        assert first == {"N": 0}
        remaining = dict_cursor.fetchall()
        assert remaining == [{"N": i} for i in range(1, 5)]


class TestDictCursorIteration:
    """Test DictCursor iteration."""

    def test_dict_cursor_is_iterable(self, dict_cursor):
        """Test DictCursor can be iterated to get dicts."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        rows = list(dict_cursor)
        assert len(rows) == 5
        assert all(isinstance(row, dict) for row in rows)
        assert rows == [{"N": i} for i in range(5)]

    def test_mixed_fetchone_and_iteration(self, dict_cursor):
        """Test mixing fetchone and iteration with DictCursor."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 5))
            ORDER BY 1
            """
        )
        first = dict_cursor.fetchone()
        assert first == {"N": 0}
        remaining = list(dict_cursor)
        assert remaining == [{"N": i} for i in range(1, 5)]


class TestDictCursorLargeResults:
    """Test DictCursor with large result sets spanning multiple batches."""

    N_ROWS = 5_000

    @pytest.mark.parametrize("data_size", [N_ROWS, 20_000])
    def test_large_result_fetchall(self, dict_cursor, data_size):
        """Test fetchall with large results returns dicts."""
        dict_cursor.execute(
            f"""
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => {data_size}))
            ORDER BY 1
            """
        )
        result = dict_cursor.fetchall()
        assert len(result) == data_size
        assert all(isinstance(row, dict) for row in result)
        assert all(row["N"] == i for i, row in enumerate(result))

    def test_large_result_iteration(self, dict_cursor):
        """Test iteration over large results returns dicts."""
        dict_cursor.execute(
            f"""
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))
            ORDER BY 1
            """
        )
        rows = list(dict_cursor)
        assert len(rows) == self.N_ROWS
        assert all(isinstance(row, dict) for row in rows)
        assert all(row["N"] == i for i, row in enumerate(rows))

    def test_large_result_multiple_columns(self, dict_cursor):
        """Test large result with multiple columns as dicts."""
        dict_cursor.execute(
            f"""
            WITH base AS (
                SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS id
                FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))
            )
            SELECT id, id * 2 AS doubled, id % 10 AS mod10 FROM base
            ORDER BY 1
            """
        )
        result = dict_cursor.fetchall()
        assert len(result) == self.N_ROWS
        assert all(isinstance(row, dict) for row in result)
        for i, row in enumerate(result):
            assert row["ID"] == i
            assert row["DOUBLED"] == i * 2
            assert row["MOD10"] == i % 10

    def test_partial_batch_consumption(self, dict_cursor):
        """Test partial consumption of batches with DictCursor."""
        dict_cursor.execute(
            f"""
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))
            ORDER BY 1
            """
        )
        consumed = self.N_ROWS // 10
        for i in range(consumed):
            row = dict_cursor.fetchone()
            assert row == {"N": i}
        remaining = dict_cursor.fetchall()
        assert len(remaining) == self.N_ROWS - consumed
        assert all(isinstance(row, dict) for row in remaining)


class TestDictCursorMultipleQueries:
    """Test DictCursor with multiple sequential queries."""

    def test_sequential_queries(self, dict_cursor):
        """Test sequential queries on same DictCursor."""
        dict_cursor.execute("SELECT 1 AS val")
        result1 = dict_cursor.fetchone()
        assert result1 == {"VAL": 1}

        dict_cursor.execute("SELECT 2 AS a, 3 AS b")
        result2 = dict_cursor.fetchone()
        assert result2 == {"A": 2, "B": 3}

    def test_new_query_resets_iterator(self, dict_cursor):
        """Test new query resets the iterator state for DictCursor."""
        dict_cursor.execute(
            """
            SELECT seq4() AS val FROM TABLE(GENERATOR(ROWCOUNT => 100))
            """
        )
        for _ in range(10):
            dict_cursor.fetchone()

        dict_cursor.execute("SELECT 42 AS answer")
        result = dict_cursor.fetchone()
        assert result == {"ANSWER": 42}

    def test_fetchone_fetchmany_fetchall_sequence(self, dict_cursor):
        """Test fetchone, fetchmany, and fetchall in sequence with DictCursor."""
        dict_cursor.execute(
            """
            SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 AS n
            FROM TABLE(GENERATOR(ROWCOUNT => 20))
            ORDER BY n
            """
        )
        row1 = dict_cursor.fetchone()
        assert row1 == {"N": 0}

        batch = dict_cursor.fetchmany(5)
        assert batch == [{"N": i} for i in range(1, 6)]

        remainder = dict_cursor.fetchall()
        assert remainder == [{"N": i} for i in range(6, 20)]
