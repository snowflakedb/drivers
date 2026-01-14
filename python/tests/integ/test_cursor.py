"""
Integration tests for PEP 249 Cursor objects.
"""

import pytest

from snowflake.ud_connector.exceptions import NotSupportedError
from decimal import Decimal


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
    def test_fetchmany_not_implemented(self, cursor):
        """Test that fetchmany raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany()
        assert "fetchmany is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference
    def test_fetchmany_with_size_not_implemented(self, cursor):
        """Test that fetchmany with size raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            cursor.fetchmany(5)
        assert "fetchmany is not implemented" in str(excinfo.value)

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

    # TODO: SNOW-2997748 - test fetchone and fetchall without execute

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
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 10))")
        result = cursor.fetchall()
        assert result == [(i,) for i in range(10)]

    def test_fetchall_multiple_columns(self, cursor):
        """Test fetchall with multiple columns."""
        cursor.execute("SELECT 1, 'hello', 3.14")
        result = cursor.fetchall()
        assert result == [(1, "hello", Decimal("3.14"))]

    @pytest.mark.skip("TODO: Known issue, SNOW-2997744")
    def test_fetchall_empty_result(self, cursor):
        """Test fetchall with empty result."""
        cursor.execute("SELECT 1 WHERE FALSE")
        result = cursor.fetchall()
        assert result == []


class TestCursorIteration:
    """Test cursor iteration."""

    def test_cursor_is_iterable(self, cursor):
        """Test cursor can be iterated."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 5))")
        rows = list(cursor)
        assert rows == [(i,) for i in range(5)]

    def test_cursor_iteration_order(self, cursor):
        """Test cursor iteration maintains order."""
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 100)) ORDER BY n DESC"
        )
        rows = list(cursor)
        for i, row in enumerate(rows):
            assert row == (99 - i,), f"Expected ({99 - i},), got {row}"

    def test_mixed_fetchone_and_iteration(self, cursor):
        """Test mixing fetchone and iteration."""
        cursor.execute("SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => 5)) ORDER BY 1")
        # Fetch first row
        first = cursor.fetchone()
        assert first == (0,)
        # Iterate rest
        remaining = list(cursor)
        assert remaining == [(1,), (2,), (3,), (4,)]


class TestCursorLargeResults:
    """Test cursor with large result sets."""

    N_ROWS = 20_000

    def test_large_result_fetchall(self, cursor):
        """Test fetchall with large results."""
        cursor.execute(
            f"SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))"
        )
        result = cursor.fetchall()
        assert result == [(i,) for i in range(self.N_ROWS)]

    def test_large_result_iteration(self, cursor):
        """Test iteration over large results."""
        cursor.execute(
            f"SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))"
        )
        rows = list(cursor)
        assert rows == [(i,) for i in range(self.N_ROWS)]

    def test_large_result_with_multiple_columns(self, cursor):
        """Test large result with multiple columns."""
        cursor.execute(
            f"""
            SELECT 
                seq4() as id,
                seq4() * 2 as doubled,
                seq4() % 10 as mod10
            FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))
        """
        )
        result = cursor.fetchall()
        assert result == [(i, i * 2, i % 10) for i in range(self.N_ROWS)]

    def test_partial_batch_consumption(self, cursor):
        """Test partial consumption of batches."""
        cursor.execute(
            f"SELECT seq4() FROM TABLE(GENERATOR(ROWCOUNT => {self.N_ROWS}))"
        )
        # Fetch only some rows
        for _ in range(self.N_ROWS // 10):
            cursor.fetchone()
        # Fetch remaining
        remaining = cursor.fetchall()
        assert remaining == [(i,) for i in range(self.N_ROWS // 10, self.N_ROWS)]


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
        cursor.execute(
            "SELECT seq4() as n FROM TABLE(GENERATOR(ROWCOUNT => 10)) ORDER BY n"
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


class TestCursorDictResult:
    """Test dict result mode.

    Note: DictCursor is not yet implemented. These tests use ArrowStreamIterator
    directly to verify dict result functionality works correctly.
    """

    @pytest.mark.skip_reference
    def test_next_returns_dict(self, cursor):
        """Test next() returns dict with column names as keys."""
        # TODO: Replace with DictCursor when implemented
        from snowflake.ud_connector._internal.arrow_stream_iterator import (
            ArrowStreamIterator,
        )
        from snowflake.ud_connector._internal.arrow_context import ArrowConverterContext

        cursor.execute("SELECT 1 AS id, 'hello' AS name")
        stream_ptr = cursor._get_stream_ptr()
        arrow_context = ArrowConverterContext()
        iterator = ArrowStreamIterator(stream_ptr, arrow_context, use_dict_result=True)

        result = next(iterator)
        assert result == {"ID": 1, "NAME": "hello"}

    @pytest.mark.skip_reference
    def test_dict_result_large_result(self, cursor):
        """Test dict result with large result set spanning multiple batches."""
        # TODO: Replace with DictCursor when implemented
        from snowflake.ud_connector._internal.arrow_stream_iterator import (
            ArrowStreamIterator,
        )
        from snowflake.ud_connector._internal.arrow_context import ArrowConverterContext

        cursor.execute(
            """
            SELECT 
                seq4() AS id,
                seq4() * 2 AS doubled
            FROM TABLE(GENERATOR(ROWCOUNT => 5000))
        """
        )
        stream_ptr = cursor._get_stream_ptr()
        arrow_context = ArrowConverterContext()
        iterator = ArrowStreamIterator(stream_ptr, arrow_context, use_dict_result=True)

        result = list(iterator)
        assert len(result) == 5000
        assert all(isinstance(row, dict) for row in result)
        assert all(len(row) == 2 for row in result)
