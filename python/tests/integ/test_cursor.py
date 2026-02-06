"""
Integration tests for PEP 249 Cursor objects.
"""

from decimal import Decimal

import pytest

from snowflake.connector.errors import NotSupportedError, ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER
from tests.e2e.types.utils import assert_sequential_values


if IS_UNIVERSAL_DRIVER:
    from snowflake.connector import SnowflakeCursor
    from snowflake.connector.cursor import SnowflakeCursorBase
else:
    from snowflake.connector.cursor import SnowflakeCursor, SnowflakeCursorBase


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

    @pytest.mark.skip("TODO: Known issue, SNOW-2997744")
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

    @pytest.mark.skip_reference
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
        if IS_UNIVERSAL_DRIVER:
            from snowflake.connector import DictCursor
        else:
            from snowflake.connector.cursor import DictCursor

        with connection.cursor(DictCursor) as cur:
            assert isinstance(cur, DictCursor)

    def test_dict_cursor_is_base_cursor_subclass(self):
        """Test that DictCursor is a subclass of BaseCursor."""
        if IS_UNIVERSAL_DRIVER:
            from snowflake.connector import DictCursor
        else:
            from snowflake.connector.cursor import DictCursor

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
