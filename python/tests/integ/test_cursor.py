"""
Integration tests for PEP 249 Cursor objects.
"""

import pytest
from decimal import Decimal
from snowflake.ud_connector.exceptions import NotSupportedError


class TestCursorMethods:
    """Test Cursor object methods."""

    def test_close_cursor(self, cursor):
        """Test closing a cursor."""
        assert not cursor.is_closed()
        cursor.close()
        assert cursor.is_closed()

    @pytest.mark.skip_reference
    def test_callproc_not_implemented(self, cursor):
        """Test that callproc raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.callproc("test_proc", [1, 2, 3])
        assert "callproc is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
    def test_executemany_not_implemented(self, cursor):
        """Test that executemany raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.executemany("INSERT INTO test VALUES (?)", [(1,), (2,)])
        assert "executemany is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
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
        cursor.execute(
            f"SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => {data_size})) v ORDER BY id"
        )
        rows = cursor.fetchall()
        assert len(rows) == data_size

        for i, row in enumerate(rows):
            assert row == (i,)


class TestCursorFetch:
    """Test cursor fetch operations."""

    def test_fetchone_single_value(self, cursor):
        """Test fetchone with a single value."""
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        assert result is not None
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

    def test_fetchall_multiple_rows(self, cursor):
        """Test fetchall with multiple rows."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 10))")
        result = cursor.fetchall()
        assert result == [(0,), (1,), (2,), (3,), (4,), (5,), (6,), (7,), (8,), (9,)]

    def test_fetchall_empty_result(self, cursor):
        """Test fetchall with empty result."""
        cursor.execute("SELECT 1 WHERE FALSE")
        result = cursor.fetchall()
        assert result == []

    def test_fetchmany_default_size(self, cursor):
        """Test fetchmany with default arraysize."""
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY n"
        )
        cursor.arraysize = 3
        result = cursor.fetchmany()
        assert result == [(0,), (1,), (2,)]

    def test_fetchmany_with_size(self, cursor):
        """Test fetchmany with explicit size."""
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY n"
        )
        result = cursor.fetchmany(5)
        assert len(result) == 5
        assert result == [(0,), (1,), (2,), (3,), (4,)]

    def test_fetchmany_returns_remaining_when_fewer_rows(self, cursor):
        """Test fetchmany returns remaining rows when fewer than size."""
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 3)) ORDER BY n"
        )
        result = cursor.fetchmany(10)
        assert len(result) == 3
        assert result == [(0,), (1,), (2,)]

    def test_fetchmany_returns_empty_when_exhausted(self, cursor):
        """Test fetchmany returns empty list when no more rows."""
        cursor.execute("SELECT 1")
        cursor.fetchall()  # Exhaust all rows
        result = cursor.fetchmany(5)
        assert result == []

    def test_fetchmany_multiple_calls(self, cursor):
        """Test multiple fetchmany calls."""
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY n"
        )
        result1 = cursor.fetchmany(3)
        result2 = cursor.fetchmany(3)
        result3 = cursor.fetchmany(3)
        result4 = cursor.fetchmany(3)  # Only 1 left

        assert result1 == [(0,), (1,), (2,)]
        assert result2 == [(3,), (4,), (5,)]
        assert result3 == [(6,), (7,), (8,)]
        assert result4 == [(9,)]


class TestCursorIteration:
    """Test cursor iteration."""

    def test_cursor_is_iterable(self, cursor):
        """Test cursor can be iterated."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 5))")
        rows = list(cursor)
        assert len(rows) == 5

    def test_cursor_iteration_order(self, cursor):
        """Test cursor iteration maintains order."""
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 100)) ORDER BY n"
        )
        rows = list(cursor)
        for i, row in enumerate(rows):
            assert row == (i,), f"Expected ({i},), got {row}"

    def test_mixed_fetchone_and_iteration(self, cursor):
        """Test mixing fetchone and iteration."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY 1")
        # Fetch first row
        first = cursor.fetchone()
        assert first == (0,)
        # Iterate rest
        remaining = list(cursor)
        assert len(remaining) == 4
        assert remaining[0] == (1,)


class TestCursorLargeResults:
    """Test cursor with large result sets."""

    @pytest.mark.parametrize("row_count", [100, 1000, 5000])
    def test_large_result_fetchall(self, cursor, row_count):
        """Test fetchall with large results."""
        cursor.execute(f"SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => {row_count}))")
        result = cursor.fetchall()
        assert len(result) == row_count

    @pytest.mark.parametrize("row_count", [100, 1000])
    def test_large_result_iteration(self, cursor, row_count):
        """Test iteration over large results."""
        cursor.execute(f"SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => {row_count}))")
        count = sum(1 for _ in cursor)
        assert count == row_count

    def test_large_result_with_multiple_columns(self, cursor):
        """Test large result with multiple columns."""
        cursor.execute(
            """
            SELECT 
                seq4() as id,
                seq4() * 2 as doubled,
                seq4() % 10 as mod10
            FROM TABLE(GENERATOR(ROWCOUNT => 1000))
        """
        )
        result = cursor.fetchall()
        assert len(result) == 1000
        assert all(len(row) == 3 for row in result)


class TestCursorBatchHandling:
    """Test cursor batch handling."""

    def test_partial_batch_consumption(self, cursor):
        """Test partial consumption of batches."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 1000))")
        # Fetch only some rows
        for _ in range(100):
            cursor.fetchone()
        # Fetch remaining
        remaining = cursor.fetchall()
        assert len(remaining) == 900


class TestCursorRowcount:
    """Test cursor rowcount attribute."""

    def test_rowcount_after_select(self, cursor, tmp_schema):
        """Test rowcount is set after SELECT."""
        cursor.execute("SELECT 1")
        assert cursor.rowcount == 1
        table_name = f"{tmp_schema}.test_rowcount"
        cursor.execute(f"CREATE OR REPLACE TABLE {table_name} (id INT)")
        assert cursor.rowcount == 1
        cursor.execute(f"INSERT INTO {table_name} VALUES (1), (2), (3)")
        assert cursor.rowcount == 1
        cursor.execute(f"SELECT * FROM {table_name}")
        assert cursor.rowcount == 3


class TestCursorMultipleQueries:
    """Test cursor with multiple queries."""

    def test_sequential_queries(self, cursor):
        """Test sequential queries on same cursor."""
        # Before any query, rownumber should be None
        assert cursor.rownumber is None

        cursor.execute("SELECT 1")
        # After execute, before fetch, rownumber should be None (not yet fetched)
        assert cursor.rownumber is None

        result1 = cursor.fetchone()
        assert result1 == (1,)
        assert cursor.rownumber == 0

        cursor.execute("SELECT 2, 3")
        # New query should reset rownumber
        assert cursor.rownumber is None

        result2 = cursor.fetchone()
        assert result2 == (2, 3)
        assert cursor.rownumber == 0

    def test_new_query_resets_iterator(self, cursor):
        """Test new query resets the iterator state."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 100))")
        # Partially consume
        for i in range(10):
            cursor.fetchone()
            assert cursor.rownumber == i

        # New query should reset
        cursor.execute("SELECT 42")
        assert cursor.rownumber is None

        result = cursor.fetchone()
        assert result == (42,)
        assert cursor.rownumber == 0

    def test_fetchall_after_partial_fetch(self, cursor):
        """Test fetchall after partial fetchone calls."""
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY n"
        )
        assert cursor.rownumber is None

        # Fetch first 3
        r1 = cursor.fetchone()
        r2 = cursor.fetchone()
        r3 = cursor.fetchone()
        assert r1 == (0,)
        assert r2 == (1,)
        assert r3 == (2,)
        assert cursor.rownumber == 2

        # Fetch remaining
        remaining = cursor.fetchall()
        assert len(remaining) == 7
        assert remaining[0] == (3,)
        # After fetchall, rownumber should be at the last row (9)
        assert cursor.rownumber == 9


class TestCursorDictResult:
    """Test dict result mode using use_dict_result=True."""

    def test_fetchone_returns_dict(self, dict_cursor):
        """Test fetchone returns dict with column names as keys."""
        dict_cursor.execute("SELECT 1 AS col_a, 'hello' AS col_b, 3.14 AS col_c")
        result = dict_cursor.fetchone()
        assert isinstance(result, dict)
        assert result == {
            "COL_A": 1,
            "COL_B": "hello",
            "COL_C": Decimal("3.14"),
        }

    def test_fetchall_returns_list_of_dicts(self, dict_cursor):
        """Test fetchall returns list of dicts."""
        dict_cursor.execute(
            "SELECT seq4() AS id FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY id"
        )
        results = dict_cursor.fetchall()
        assert results == [{"ID": 0}, {"ID": 1}, {"ID": 2}, {"ID": 3}, {"ID": 4}]

    def test_dict_result_multiple_columns(self, dict_cursor):
        """Test dict result with multiple columns."""
        dict_cursor.execute(
            """
            SELECT 
                42 AS numeric_col,
                'test' AS string_col,
                TRUE AS bool_col
        """
        )
        result = dict_cursor.fetchone()
        assert result == {"NUMERIC_COL": 42, "STRING_COL": "test", "BOOL_COL": True}

    def test_dict_result_large_result(self, dict_cursor):
        """Test dict result with large result set spanning multiple batches."""
        dict_cursor.execute(
            """
            SELECT 
                seq4() AS id,
                seq4() * 2 AS doubled
            FROM TABLE(GENERATOR(ROWCOUNT => 1000))
            ORDER BY id
        """
        )
        results = dict_cursor.fetchall()
        assert len(results) == 1000
        assert all(isinstance(row, dict) for row in results)
        # Verify each row has the expected columns
        for row in results:
            assert len(row) == 2

    def test_dict_iteration(self, dict_cursor):
        """Test iterating over cursor returns dicts."""
        dict_cursor.execute(
            "SELECT seq4() AS n FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY n"
        )
        rows = list(dict_cursor)
        assert rows == list({"N": i} for i in range(10))

    def test_dict_mixed_fetchone_and_fetchall(self, dict_cursor):
        """Test mixing fetchone and fetchall with dict results."""
        dict_cursor.execute(
            "SELECT seq4() AS n FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY n"
        )
        # Fetch first few rows
        r1 = dict_cursor.fetchone()
        r2 = dict_cursor.fetchone()
        assert r1 == {"N": 0}
        assert r2 == {"N": 1}

        # Fetch remaining
        remaining = dict_cursor.fetchall()
        assert remaining == [{"N": 2}, {"N": 3}, {"N": 4}]

    def test_dict_should_not_crash_when_no_rows(self, dict_cursor):
        """Test dict should not crash when no rows."""
        dict_cursor.execute("SELECT 1 WHERE FALSE")
        result = dict_cursor.fetchall()
        assert result == []
