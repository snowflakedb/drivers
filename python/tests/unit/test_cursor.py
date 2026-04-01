"""
Unit tests for PEP 249 Cursor class.
"""

from decimal import Decimal
from unittest.mock import ANY, MagicMock, patch

import pytest

from snowflake.connector._internal.binding_converters import ParamStyle
from snowflake.connector._internal.errorcode import ER_NO_PYARROW
from snowflake.connector._internal.extras import (
    MissingOptionalDependency,
)
from snowflake.connector._internal.extras import (
    check_dependency as _real_check_dependency,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    StatementHandle,
)
from snowflake.connector.constants import QueryStatus
from snowflake.connector.cursor import FetchMode, QueryResultStats, SnowflakeCursor, SnowflakeCursorBase
from snowflake.connector.cursor._query_result_waiter import QueryResultWaiter
from snowflake.connector.errors import DatabaseError, InterfaceError, ProgrammingError


class TestFetchone:
    """Unit tests for Cursor.fetchone method."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchone_returns_single_row(self, cursor):
        """Test fetchone returns a single row tuple."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result == (1,)

    def test_fetchone_returns_none_when_exhausted(self, cursor):
        """Test fetchone returns None when no more rows."""
        mock_iterator = iter([])
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result is None

    def test_fetchone_sequential_calls(self, cursor):
        """Test sequential fetchone calls return rows in order."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            first = cursor.fetchone()
            second = cursor.fetchone()
            third = cursor.fetchone()
            fourth = cursor.fetchone()

        assert first == (1,)
        assert second == (2,)
        assert third == (3,)
        assert fourth is None

    def test_fetchone_calls_create_row_iterator_if_iterator_is_none(self, cursor):
        """Test fetchone calls _create_row_iterator."""
        mock_ensure = MagicMock(return_value=iter([(1,)]))

        with patch.object(cursor, "_create_row_iterator", mock_ensure):
            cursor.fetchone()

        mock_ensure.assert_called_once()

    def test_fetchone_with_multi_column_row(self, cursor):
        """Test fetchone with multiple columns."""
        mock_rows = [(1, "hello", 3.14)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result == (1, "hello", 3.14)

    def test_fetchone_preserves_types(self, cursor):
        """Test fetchone preserves data types."""
        mock_rows = [(1, "text", Decimal("3.14"), None, True)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result[0] == 1
        assert result[1] == "text"
        assert result[2] == Decimal("3.14")
        assert isinstance(result[2], Decimal)
        assert result[3] is None
        assert result[4] is True

    def test_fetchone_with_empty_tuple_row(self, cursor):
        """Test fetchone handles empty tuple row."""
        mock_rows = [()]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result == ()

    def test_fetchone_after_exhaustion_returns_none(self, cursor):
        """Test fetchone consistently returns None after exhaustion."""
        mock_rows = [(1,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()  # Consume the row
            result1 = cursor.fetchone()
            result2 = cursor.fetchone()

        assert result1 is None
        assert result2 is None


class TestFetchall:
    """Unit tests for Cursor.fetchall method."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchall_returns_all_rows(self, cursor):
        """Test fetchall returns all rows as a list."""
        mock_rows = [(1,), (2,), (3,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == [(1,), (2,), (3,)]

    def test_fetchall_returns_empty_list_when_no_rows(self, cursor):
        """Test fetchall returns empty list when no rows."""
        cursor._iterator = iter([])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_calls_create_row_iterator_if_iterator_is_none(self, cursor):
        """Test fetchall calls _create_row_iterator."""
        mock_ensure = MagicMock()

        with patch.object(cursor, "_create_row_iterator", mock_ensure):
            cursor.fetchall()

        mock_ensure.assert_called_once()

    def test_fetchall_with_single_row(self, cursor):
        """Test fetchall with single row."""
        mock_rows = [(42,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == [(42,)]
        assert len(result) == 1

    def test_fetchall_with_multi_column_rows(self, cursor):
        """Test fetchall with multiple columns per row."""
        mock_rows = [
            (1, "a", 1.0),
            (2, "b", 2.0),
            (3, "c", 3.0),
        ]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == [(1, "a", 1.0), (2, "b", 2.0), (3, "c", 3.0)]

    def test_fetchall_preserves_types(self, cursor):
        """Test fetchall preserves data types in rows."""
        mock_rows = [
            (1, "text", Decimal("3.14"), None),
            (2, "more", Decimal("2.71"), True),
        ]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert isinstance(result[1][2], Decimal)

    def test_fetchall_after_partial_fetchone(self, cursor):
        """Test fetchall returns remaining rows after fetchone."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            # Fetch first two rows
            cursor.fetchone()
            cursor.fetchone()
            # Fetch remaining
            result = cursor.fetchall()

        assert result == [(3,), (4,), (5,)]

    def test_fetchall_returns_empty_after_exhaustion(self, cursor):
        """Test fetchall returns empty list after all rows consumed."""
        mock_rows = [(1,), (2,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchall()  # Consume all rows
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_with_large_result_set(self, cursor):
        """Test fetchall with large number of rows."""
        mock_rows = [(i,) for i in range(1000)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert len(result) == 1000
        assert result[0] == (0,)
        assert result[999] == (999,)

    def test_fetchall_returns_list_not_iterator(self, cursor):
        """Test fetchall returns a list, not an iterator."""
        mock_rows = [(1,), (2,), (3,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert isinstance(result, list)


class TestFetchmany:
    """Unit tests for Cursor.fetchmany method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchmany_default_uses_arraysize(self, cursor):
        """Test that fetchmany() without size argument uses arraysize."""
        cursor.arraysize = 3
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany()

        assert result == [(1,), (2,), (3,)]

    def test_fetchmany_with_explicit_size(self, cursor):
        """Test fetchmany with explicit size argument."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(2)

        assert result == [(1,), (2,)]

    def test_fetchmany_returns_fewer_rows_when_exhausted(self, cursor):
        """Test fetchmany returns fewer rows when result set is exhausted."""
        mock_rows = [(1,), (2,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(5)

        assert result == [(1,), (2,)]

    def test_fetchmany_returns_empty_list_when_no_rows(self, cursor):
        """Test fetchmany returns empty list when no rows available."""
        with patch.object(cursor, "fetchone", return_value=None):
            result = cursor.fetchmany(5)

        assert result == []

    def test_fetchmany_with_size_zero(self, cursor):
        """Test fetchmany(0) returns empty list."""
        mock_fetchone = MagicMock()
        with patch.object(cursor, "fetchone", mock_fetchone):
            result = cursor.fetchmany(0)

        assert result == []
        mock_fetchone.assert_not_called()

    def test_fetchmany_with_negative_size_raises_error(self, cursor):
        """Test fetchmany with negative size raises ProgrammingError."""
        with pytest.raises(ProgrammingError) as excinfo:
            cursor.fetchmany(-1)

        assert "The number of rows is not zero or positive number: -1" in str(excinfo.value)

    def test_fetchmany_with_negative_size_various_values(self, cursor):
        """Test fetchmany raises ProgrammingError for various negative values."""
        with pytest.raises(ProgrammingError) as excinfo:
            cursor.fetchmany(-42)

        assert "The number of rows is not zero or positive number: -42" in str(excinfo.value)

    def test_fetchmany_sequential_calls(self, cursor):
        """Test multiple sequential fetchmany calls consume rows correctly."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            first_batch = cursor.fetchmany(2)
            second_batch = cursor.fetchmany(2)
            third_batch = cursor.fetchmany(2)

        assert first_batch == [(1,), (2,)]
        assert second_batch == [(3,), (4,)]
        assert third_batch == [(5,)]

    def test_fetchmany_after_exhausted_returns_empty(self, cursor):
        """Test fetchmany returns empty list after all rows consumed."""
        mock_rows = [(1,), (2,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            cursor.fetchmany(5)  # Consume all rows
            result = cursor.fetchmany(5)

        assert result == []

    def test_fetchmany_respects_changed_arraysize(self, cursor):
        """Test fetchmany respects dynamically changed arraysize."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,), (6,), (7,), (8,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            cursor.arraysize = 2
            first_batch = cursor.fetchmany()

            cursor.arraysize = 4
            second_batch = cursor.fetchmany()

        assert first_batch == [(1,), (2,)]
        assert second_batch == [(3,), (4,), (5,), (6,)]

    def test_fetchmany_with_size_one(self, cursor):
        """Test fetchmany(1) returns single row list."""
        mock_rows = [(1,), (2,), (3,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(1)

        assert result == [(1,)]

    def test_fetchmany_with_large_size(self, cursor):
        """Test fetchmany with size larger than available rows."""
        mock_rows = [(i,) for i in range(10)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(1000)

        assert result == [(i,) for i in range(10)]

    def test_fetchmany_default_arraysize_is_one(self, cursor):
        """Test that default arraysize is 1."""
        assert cursor.arraysize == 1

        mock_rows = [(1,), (2,), (3,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany()

        # Default arraysize is 1, so should fetch 1 row
        assert result == [(1,)]

    def test_fetchmany_with_multi_column_rows(self, cursor):
        """Test fetchmany with rows containing multiple columns."""
        mock_rows = [
            (1, "a", 1.0),
            (2, "b", 2.0),
            (3, "c", 3.0),
        ]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(2)

        assert result == [(1, "a", 1.0), (2, "b", 2.0)]

    def test_fetchmany_preserves_row_types(self, cursor):
        """Test that fetchmany preserves the types in rows."""
        mock_rows = [
            (1, "text", Decimal("3.14"), None),
            (2, "more", Decimal("2.71"), True),
        ]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(2)

        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert result[0][3] is None
        assert result[1][3] is True


class TestStatementLifecycle:
    """Unit tests for statement handle lifecycle (create/release).

    Each execute() creates a statement handle, runs the query, then
    releases the handle immediately — the Arrow stream returned by
    execute is fully owned and does not reference the handle.
    """

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection with db_api stubs for execute flow."""
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        handle_counter = 0

        def new_handle(*_args, **_kwargs):
            nonlocal handle_counter
            handle_counter += 1
            resp = MagicMock()
            resp.stmt_handle = StatementHandle(id=handle_counter)
            return resp

        conn.db_api.statement_new.side_effect = new_handle
        execute_result = MagicMock()
        execute_result.columns = []
        execute_result.HasField = MagicMock(return_value=False)
        conn.db_api.statement_execute_query.return_value.result = execute_result
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with the mocked connection."""
        return SnowflakeCursor(mock_connection)

    def test_execute_releases_handle_immediately(self, cursor, mock_connection):
        """A single execute must release its statement handle before returning."""
        cursor.execute("SELECT 1")

        mock_connection.db_api.statement_release.assert_called_once()
        released_id = mock_connection.db_api.statement_release.call_args.args[0].stmt_handle.id
        assert released_id == 1

    def test_sequential_executes_release_all_handles(self, cursor, mock_connection):
        """Every execute creates and releases its own handle — nothing leaks."""
        n = 5
        for i in range(n):
            cursor.execute(f"SELECT {i}")

        release = mock_connection.db_api.statement_release
        assert release.call_count == n

        released_ids = [call.args[0].stmt_handle.id for call in release.call_args_list]
        assert released_ids == list(range(1, n + 1))

    def test_close_without_execute_does_not_release(self, cursor, mock_connection):
        """Closing a cursor that never executed should not call release."""
        cursor.close()

        mock_connection.db_api.statement_release.assert_not_called()


class TestSqlstate:
    """Unit tests for Cursor.sqlstate property."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.db_api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sqlstate_none_before_execute(self, cursor):
        """sqlstate is None on a fresh cursor."""
        assert cursor.sqlstate is None

    def test_sqlstate_none_after_successful_execute(self, cursor, mock_connection):
        """sqlstate is None when server returns '00000' (successful completion)."""
        result = MagicMock()
        result.columns = []
        result.sql_state = "00000"
        mock_connection.db_api.statement_execute_query.return_value.result = result

        cursor.execute("SELECT 1")

        assert cursor.sqlstate is None

    def test_sqlstate_populated_with_error_code(self, cursor, mock_connection):
        """sqlstate reflects non-success sql_state from execute result."""
        result = MagicMock()
        result.columns = []
        result.sql_state = "42601"
        mock_connection.db_api.statement_execute_query.return_value.result = result

        cursor.execute("SELECT 1")

        assert cursor.sqlstate == "42601"

    def test_sqlstate_none_when_field_absent(self, cursor, mock_connection):
        """sqlstate is None when the server does not return sql_state."""
        result = MagicMock()
        result.columns = []
        result.sql_state = ""
        mock_connection.db_api.statement_execute_query.return_value.result = result

        cursor.execute("SELECT 1")

        assert cursor.sqlstate is None

    def test_sqlstate_updates_on_subsequent_execute(self, cursor, mock_connection):
        """sqlstate is refreshed on every execute call."""
        first_result = MagicMock()
        first_result.columns = []
        first_result.sql_state = "42601"

        second_result = MagicMock()
        second_result.columns = []
        second_result.sql_state = "00000"

        mock_connection.db_api.statement_execute_query.return_value.result = first_result
        cursor.execute("SELECT 1")
        assert cursor.sqlstate == "42601"

        mock_connection.db_api.statement_execute_query.return_value.result = second_result
        cursor.execute("SELECT 2")
        assert cursor.sqlstate is None

    def test_sqlstate_set_from_error_on_failed_execute(self, cursor, mock_connection):
        """sqlstate is captured from PEP 249 Error when execute raises."""
        mock_connection.db_api.statement_execute_query.side_effect = ProgrammingError("error", sqlstate="42601")

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sqlstate == "42601"

    def test_sqlstate_set_to_none_when_error_has_no_sqlstate(self, cursor, mock_connection):
        """sqlstate is set to None when error carries no sqlstate."""
        mock_connection.db_api.statement_execute_query.side_effect = ProgrammingError("error", sqlstate=None)

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sqlstate is None

    def test_sqlstate_transitions_across_success_and_failure(self, cursor, mock_connection):
        """sqlstate updates correctly through None -> error -> None."""
        success_result = MagicMock()
        success_result.columns = []
        success_result.sql_state = "00000"

        mock_connection.db_api.statement_execute_query.return_value.result = success_result
        mock_connection.db_api.statement_execute_query.side_effect = None
        cursor.execute("SELECT 1")
        assert cursor.sqlstate is None

        mock_connection.db_api.statement_execute_query.side_effect = ProgrammingError("error", sqlstate="42601")
        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")
        assert cursor.sqlstate == "42601"

        mock_connection.db_api.statement_execute_query.side_effect = None
        mock_connection.db_api.statement_execute_query.return_value.result = success_result
        cursor.execute("SELECT 2")
        assert cursor.sqlstate is None


class TestSfqidOnFailedQuery:
    """Unit tests for cursor.sfqid propagation when execute raises."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.db_api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sfqid_set_from_error_on_failed_execute(self, cursor, mock_connection):
        """sfqid is captured from ProgrammingError when execute raises."""
        mock_connection.db_api.statement_execute_query.side_effect = ProgrammingError("error", sfqid="01abc-def-12345")

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sfqid == "01abc-def-12345"

    def test_sfqid_none_when_error_has_no_sfqid(self, cursor, mock_connection):
        """sfqid is None when error carries no sfqid."""
        mock_connection.db_api.statement_execute_query.side_effect = ProgrammingError("error")

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sfqid is None


class TestQueryResultStats:
    """Unit tests for QueryResultStats NamedTuple."""

    def test_default_all_none(self):
        """All fields default to None."""
        stats = QueryResultStats()
        assert stats.num_rows_inserted is None
        assert stats.num_rows_deleted is None
        assert stats.num_rows_updated is None
        assert stats.num_dml_duplicates is None

    def test_positional_construction(self):
        """Fields can be set by position."""
        stats = QueryResultStats(10, 20, 30, 5)
        assert stats.num_rows_inserted == 10
        assert stats.num_rows_deleted == 20
        assert stats.num_rows_updated == 30
        assert stats.num_dml_duplicates == 5

    def test_keyword_construction(self):
        """Fields can be set by keyword."""
        stats = QueryResultStats(num_rows_inserted=1, num_rows_updated=2)
        assert stats.num_rows_inserted == 1
        assert stats.num_rows_deleted is None
        assert stats.num_rows_updated == 2
        assert stats.num_dml_duplicates is None

    def test_is_named_tuple(self):
        """QueryResultStats is a proper NamedTuple with tuple semantics."""
        stats = QueryResultStats(1, 2, 3, 4)
        assert isinstance(stats, tuple)
        assert len(stats) == 4
        assert stats[0] == 1
        assert stats._fields == ("num_rows_inserted", "num_rows_deleted", "num_rows_updated", "num_dml_duplicates")

    def test_equality(self):
        """Two instances with identical values are equal."""
        a = QueryResultStats(1, 2, 3, 4)
        b = QueryResultStats(1, 2, 3, 4)
        assert a == b

    def test_all_none_equality(self):
        """Default instance equals explicit all-None instance."""
        assert QueryResultStats() == QueryResultStats(None, None, None, None)

    def test_from_query_stats_all_fields_present(self):
        """from_query_stats maps all present protobuf fields."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 10
        mock_stats.num_rows_deleted = 5
        mock_stats.num_rows_updated = 3
        mock_stats.num_dml_duplicates = 1
        mock_stats.HasField.return_value = True

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats(10, 5, 3, 1)

    def test_from_query_stats_partial_fields(self):
        """from_query_stats returns None for absent protobuf fields."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 42
        mock_stats.HasField.side_effect = lambda name: name == "num_rows_inserted"

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats(
            num_rows_inserted=42, num_rows_deleted=None, num_rows_updated=None, num_dml_duplicates=None
        )

    def test_from_query_stats_no_fields_present(self):
        """from_query_stats returns all None when no fields are set."""
        mock_stats = MagicMock()
        mock_stats.HasField.return_value = False

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats()

    def test_from_query_stats_zero_values(self):
        """from_query_stats preserves zero values (distinct from absent)."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 0
        mock_stats.num_rows_deleted = 0
        mock_stats.num_rows_updated = 0
        mock_stats.num_dml_duplicates = 0
        mock_stats.HasField.return_value = True

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats(0, 0, 0, 0)


class TestStats:
    """Unit tests for Cursor.stats property."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_stats_returns_all_none_before_execute(self, cursor):
        """stats returns QueryResultStats with all None fields on a fresh cursor."""
        result = cursor.stats
        assert result == QueryResultStats(None, None, None, None)

    def test_stats_returns_all_none_when_no_stats_field(self, cursor):
        """stats returns all-None when execute_result has no stats field."""
        mock_result = MagicMock()
        mock_result.HasField.return_value = False
        cursor._execute_result = mock_result

        result = cursor.stats

        assert result == QueryResultStats(None, None, None, None)
        mock_result.HasField.assert_called_with("stats")

    def test_stats_returns_all_fields_when_present(self, cursor):
        """stats returns all populated fields from execute_result."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 10
        mock_stats.num_rows_deleted = 5
        mock_stats.num_rows_updated = 3
        mock_stats.num_dml_duplicates = 1
        mock_stats.HasField.return_value = True

        mock_result = MagicMock()
        mock_result.HasField.return_value = True
        mock_result.stats = mock_stats
        cursor._execute_result = mock_result

        result = cursor.stats

        assert result == QueryResultStats(
            num_rows_inserted=10,
            num_rows_deleted=5,
            num_rows_updated=3,
            num_dml_duplicates=1,
        )

    def test_stats_returns_partial_fields(self, cursor):
        """stats returns None for fields not present in the protobuf."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 10
        mock_stats.num_rows_deleted = 0
        mock_stats.num_rows_updated = 0
        mock_stats.num_dml_duplicates = 0

        def has_field(name):
            return name == "num_rows_inserted"

        mock_stats.HasField.side_effect = has_field

        mock_result = MagicMock()
        mock_result.HasField.return_value = True
        mock_result.stats = mock_stats
        cursor._execute_result = mock_result

        result = cursor.stats

        assert result.num_rows_inserted == 10
        assert result.num_rows_deleted is None
        assert result.num_rows_updated is None
        assert result.num_dml_duplicates is None

    def test_stats_distinguishes_zero_from_absent(self, cursor):
        """A field present with value 0 is returned as 0, not None."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 0
        mock_stats.num_rows_deleted = 0
        mock_stats.num_rows_updated = 0
        mock_stats.num_dml_duplicates = 0
        mock_stats.HasField.return_value = True

        mock_result = MagicMock()
        mock_result.HasField.return_value = True
        mock_result.stats = mock_stats
        cursor._execute_result = mock_result

        result = cursor.stats

        assert result == QueryResultStats(0, 0, 0, 0)

    def test_stats_returns_query_result_stats_type(self, cursor):
        """stats always returns a QueryResultStats instance."""
        assert isinstance(cursor.stats, QueryResultStats)

        mock_result = MagicMock()
        mock_result.HasField.return_value = False
        cursor._execute_result = mock_result
        assert isinstance(cursor.stats, QueryResultStats)

    def test_stats_updates_on_subsequent_execute(self, cursor):
        """stats reflects the most recent execute_result."""
        first_stats = MagicMock()
        first_stats.num_rows_inserted = 5
        first_stats.HasField.return_value = True

        first_result = MagicMock()
        first_result.HasField.return_value = True
        first_result.stats = first_stats
        cursor._execute_result = first_result

        assert cursor.stats.num_rows_inserted == 5

        second_stats = MagicMock()
        second_stats.num_rows_inserted = 20
        second_stats.HasField.return_value = True

        second_result = MagicMock()
        second_result.HasField.return_value = True
        second_result.stats = second_stats
        cursor._execute_result = second_result

        assert cursor.stats.num_rows_inserted == 20

    def test_stats_only_insert_field(self, cursor):
        """stats correctly returns only num_rows_inserted when only that field is present."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 42

        def has_field(name):
            return name == "num_rows_inserted"

        mock_stats.HasField.side_effect = has_field

        mock_result = MagicMock()
        mock_result.HasField.return_value = True
        mock_result.stats = mock_stats
        cursor._execute_result = mock_result

        result = cursor.stats
        assert result == QueryResultStats(
            num_rows_inserted=42, num_rows_deleted=None, num_rows_updated=None, num_dml_duplicates=None
        )


class TestFetchmanyArraysizeAttribute:
    """Tests for arraysize attribute interaction with fetchmany."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_arraysize_default(self, cursor):
        """Test that cursor has default arraysize of 1."""
        assert cursor.arraysize == 1

    def test_arraysize_is_property(self):
        """Test that arraysize is a property on the class."""
        assert isinstance(SnowflakeCursorBase.__dict__["arraysize"], property)

    def test_arraysize_instance_independent(self, cursor):
        """Test instance arraysize changes are independent."""
        assert cursor.arraysize == 1
        cursor.arraysize = 10
        assert cursor.arraysize == 10

    def test_fetchmany_uses_instance_arraysize(self, cursor):
        """Test fetchmany uses instance arraysize, not class attribute."""
        cursor.arraysize = 5
        mock_rows = [(i,) for i in range(10)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany()

        assert len(result) == 5


class TestRownumber:
    """Unit tests for Cursor.rownumber property."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_rownumber_none_before_fetch(self, cursor):
        """rownumber is None before any rows have been fetched."""
        assert cursor.rownumber is None

    def test_rownumber_increments_with_fetchone(self, cursor):
        """rownumber increments by 1 for each fetchone call."""
        cursor._iterator = iter([(1,), (2,), (3,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0
            cursor.fetchone()
            assert cursor.rownumber == 1
            cursor.fetchone()
            assert cursor.rownumber == 2

    def test_rownumber_stays_after_fetchone_exhausted(self, cursor):
        """rownumber stays at last value when fetchone returns None."""
        cursor._iterator = iter([(1,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0
            cursor.fetchone()  # returns None
            assert cursor.rownumber == 0

    def test_rownumber_updated_by_fetchall(self, cursor):
        """rownumber reflects total rows fetched after fetchall."""
        cursor._iterator = iter([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchall()
            assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchall_after_partial_fetchone(self, cursor):
        """rownumber is correct when fetchall follows partial fetchone consumption."""
        cursor._iterator = iter([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            cursor.fetchone()
            assert cursor.rownumber == 1
            cursor.fetchall()
            assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchmany(self, cursor):
        """rownumber increments correctly through fetchmany calls."""
        cursor._iterator = iter([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchmany(3)
            assert cursor.rownumber == 2
            cursor.fetchmany(2)
            assert cursor.rownumber == 4

    def test_rownumber_fetchall_on_empty_result(self, cursor):
        """rownumber stays None when fetchall returns no rows."""
        cursor._iterator = iter([])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchall()
            assert cursor.rownumber is None

    def test_rownumber_none_after_execute_resets(self, cursor):
        """rownumber resets to None after a new execute call."""
        cursor._iterator = iter([(1,), (2,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0

        cursor._rownumber = -1  # simulates what execute() does
        assert cursor.rownumber is None


class TestCreateRowIteratorNumpyFlag:
    """Unit tests for _create_row_iterator passing connection._numpy."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.is_closed.return_value = False
        return conn

    def test_passes_numpy_true_from_connection(self, mock_connection):
        mock_connection._numpy = True
        cursor = SnowflakeCursor(mock_connection)
        cursor._execute_result = MagicMock()

        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_row_iterator") as mock_create,
        ):
            mock_create.return_value = iter([])
            cursor._create_row_iterator()

        mock_create.assert_called_once_with(
            stream_ptr=42,
            use_dict_result=False,
            use_numpy=True,
        )

    def test_passes_numpy_false_from_connection(self, mock_connection):
        mock_connection._numpy = False
        cursor = SnowflakeCursor(mock_connection)
        cursor._execute_result = MagicMock()

        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_row_iterator") as mock_create,
        ):
            mock_create.return_value = iter([])
            cursor._create_row_iterator()

        mock_create.assert_called_once_with(
            stream_ptr=42,
            use_dict_result=False,
            use_numpy=False,
        )


class TestCheckCanUseArrowResultset:
    """Unit tests for SnowflakeCursorBase.check_can_use_arrow_resultset."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_no_error_when_pyarrow_installed(self, cursor):
        """check_can_use_arrow_resultset does not raise when pyarrow is available."""
        with patch("snowflake.connector.cursor._base.pyarrow", MagicMock()):
            cursor.check_can_use_arrow_resultset()

    def test_raises_programming_error_when_pyarrow_missing(self, cursor):
        """check_can_use_arrow_resultset raises ProgrammingError when pyarrow is not installed."""
        with patch("snowflake.connector.cursor._base.pyarrow", MissingOptionalDependency(dep="pyarrow")):
            with pytest.raises(ProgrammingError) as excinfo:
                cursor.check_can_use_arrow_resultset()
            assert excinfo.value.errno == ER_NO_PYARROW
            assert "pyarrow" in str(excinfo.value)

    def test_error_message_contains_install_link(self, cursor):
        """The error message includes the documentation link for installation."""
        with patch("snowflake.connector.cursor._base.pyarrow", MissingOptionalDependency(dep="pyarrow")):
            with pytest.raises(ProgrammingError, match="python-connector-pandas"):
                cursor.check_can_use_arrow_resultset()


class TestCheckCanUsePandas:
    """Unit tests for SnowflakeCursorBase.check_can_use_pandas."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_no_error_when_pandas_installed(self, cursor):
        """check_can_use_pandas does not raise when pandas is available."""
        with patch("snowflake.connector.cursor._base.pandas", MagicMock()):
            cursor.check_can_use_pandas()

    def test_raises_programming_error_when_pandas_missing(self, cursor):
        """check_can_use_pandas raises ProgrammingError when pandas is not installed."""
        with patch("snowflake.connector.cursor._base.pandas", MissingOptionalDependency(dep="pandas")):
            with pytest.raises(ProgrammingError) as excinfo:
                cursor.check_can_use_pandas()
            assert excinfo.value.errno == ER_NO_PYARROW
            assert "pandas" in str(excinfo.value)

    def test_error_message_contains_install_link(self, cursor):
        """The error message includes the documentation link for installation."""
        with patch("snowflake.connector.cursor._base.pandas", MissingOptionalDependency(dep="pandas")):
            with pytest.raises(ProgrammingError, match="python-connector-pandas"):
                cursor.check_can_use_pandas()


class TestFetchArrowBatches:
    """Unit tests for fetch_arrow_batches."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_pyarrow(self):
        mock_pa = MagicMock()
        with (
            patch("snowflake.connector._internal.extras.check_dependency"),
            patch("snowflake.connector.cursor._base.pyarrow", new=mock_pa),
        ):
            self.pa = mock_pa
            yield

    def test_yields_tables_from_batches(self, cursor):
        batch1, batch2 = MagicMock(), MagicMock()
        table1, table2 = MagicMock(), MagicMock()
        self.pa.Table.from_batches.side_effect = [table1, table2]

        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([batch1, batch2])),
        ):
            tables = list(cursor.fetch_arrow_batches())

        assert tables == [table1, table2]
        self.pa.Table.from_batches.assert_any_call([batch1])
        self.pa.Table.from_batches.assert_any_call([batch2])

    def test_yields_nothing_for_empty_stream(self, cursor):
        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])),
        ):
            tables = list(cursor.fetch_arrow_batches())

        assert tables == []

    def test_raises_when_pyarrow_not_installed(self, cursor):
        missing = MissingOptionalDependency(dep="pyarrow")
        with patch(
            "snowflake.connector._internal.extras.check_dependency",
            side_effect=lambda _: _real_check_dependency(missing),
        ):
            with pytest.raises(ProgrammingError, match="pyarrow"):
                list(cursor.fetch_arrow_batches())

    def test_passes_force_microsecond_precision(self, cursor):
        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])) as mock_get,
        ):
            list(cursor.fetch_arrow_batches(force_microsecond_precision=True))

        mock_get.assert_called_once_with(stream_ptr=42, force_microsecond_precision=True, number_to_decimal=ANY)


class TestFetchArrowAll:
    """Unit tests for fetch_arrow_all."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_pyarrow(self):
        mock_pa = MagicMock()
        with (
            patch("snowflake.connector._internal.extras.check_dependency"),
            patch("snowflake.connector.cursor._base.pyarrow", new=mock_pa),
            patch("snowflake.connector._internal.arrow_stream_utils.pyarrow", new=mock_pa),
        ):
            self.pa = mock_pa
            yield

    def test_returns_concatenated_table(self, cursor):
        batch1, batch2 = MagicMock(), MagicMock()
        mock_table = MagicMock()
        self.pa.Table.from_batches.return_value = mock_table

        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([batch1, batch2])),
        ):
            result = cursor.fetch_arrow_all()

        assert result is mock_table
        self.pa.Table.from_batches.assert_called_once_with([batch1, batch2])

    def test_returns_none_for_empty_stream(self, cursor):
        mock_iterator = MagicMock()
        mock_iterator.__iter__ = MagicMock(return_value=iter([]))

        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=mock_iterator),
        ):
            result = cursor.fetch_arrow_all()

        assert result is None

    def test_returns_empty_table_with_force_return_table(self, cursor):
        mock_empty_table = MagicMock()
        mock_schema = MagicMock()
        mock_schema.empty_table.return_value = mock_empty_table

        mock_iterator = MagicMock()
        mock_iterator.__iter__ = MagicMock(return_value=iter([]))
        mock_iterator.get_converted_schema.return_value = mock_schema

        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=mock_iterator),
        ):
            result = cursor.fetch_arrow_all(force_return_table=True)

        assert result is mock_empty_table
        mock_iterator.get_converted_schema.assert_called_once()
        mock_schema.empty_table.assert_called_once()

    def test_returns_none_without_force_return_table(self, cursor):
        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])),
        ):
            result = cursor.fetch_arrow_all(force_return_table=False)

        assert result is None

    def test_passes_force_microsecond_precision(self, cursor):
        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])) as mock_get,
        ):
            cursor.fetch_arrow_all(force_microsecond_precision=True)

        mock_get.assert_called_once_with(stream_ptr=42, force_microsecond_precision=True, number_to_decimal=ANY)


class TestFetchPandasBatches:
    """Unit tests for fetch_pandas_batches."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    def test_yields_to_pandas_results(self, cursor):
        table1, table2 = MagicMock(), MagicMock()
        df1, df2 = MagicMock(), MagicMock()
        table1.to_pandas.return_value = df1
        table2.to_pandas.return_value = df2

        with patch.object(cursor, "fetch_arrow_batches", return_value=iter([table1, table2])):
            dfs = list(cursor.fetch_pandas_batches())

        assert dfs == [df1, df2]
        table1.to_pandas.assert_called_once()
        table2.to_pandas.assert_called_once()

    def test_raises_when_pandas_not_installed(self, cursor):
        missing = MissingOptionalDependency(dep="pandas")
        with patch(
            "snowflake.connector._internal.extras.check_dependency",
            side_effect=lambda _: _real_check_dependency(missing),
        ):
            with pytest.raises(ProgrammingError, match="pandas"):
                list(cursor.fetch_pandas_batches())


class TestFetchPandasAll:
    """Unit tests for fetch_pandas_all."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    def test_returns_to_pandas_result(self, cursor):
        mock_table = MagicMock()
        mock_df = MagicMock()
        mock_table.to_pandas.return_value = mock_df

        with patch.object(cursor, "fetch_arrow_all", return_value=mock_table):
            result = cursor.fetch_pandas_all()

        assert result is mock_df
        mock_table.to_pandas.assert_called_once()

    def test_returns_empty_dataframe_for_empty_stream(self, cursor):
        mock_empty_table = MagicMock()
        mock_empty_df = MagicMock()
        mock_empty_table.to_pandas.return_value = mock_empty_df

        with patch.object(cursor, "fetch_arrow_all", return_value=mock_empty_table) as mock_fetch:
            result = cursor.fetch_pandas_all()

        assert result is mock_empty_df
        mock_fetch.assert_called_once_with(force_return_table=True)
        mock_empty_table.to_pandas.assert_called_once()

    def test_raises_when_pandas_not_installed(self, cursor):
        missing = MissingOptionalDependency(dep="pandas")
        with patch(
            "snowflake.connector._internal.extras.check_dependency",
            side_effect=lambda _: _real_check_dependency(missing),
        ):
            with pytest.raises(ProgrammingError, match="pandas"):
                cursor.fetch_pandas_all()

    def test_forwards_kwargs_to_fetch_arrow_all(self, cursor):
        mock_table = MagicMock()
        with patch.object(cursor, "fetch_arrow_all", return_value=mock_table) as mock_fetch:
            cursor.fetch_pandas_all(force_microsecond_precision=True)

        mock_fetch.assert_called_once_with(force_return_table=True, force_microsecond_precision=True)


class TestFetchModeValidation:
    """Unit tests for fetch mode validation (preventing mixed row/arrow fetching)."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with (
            patch("snowflake.connector._internal.extras.check_dependency"),
            patch("snowflake.connector.cursor._base.pyarrow", new=MagicMock()),
            patch("snowflake.connector.cursor._base.pandas", new=MagicMock()),
        ):
            yield

    def test_row_then_arrow_raises(self, cursor):
        cursor._iterator = iter([(1,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()

        with pytest.raises(ProgrammingError, match="Cannot use arrow/pandas fetch methods"):
            list(cursor.fetch_arrow_batches())

    def test_arrow_then_row_raises(self, cursor):
        with (
            patch("snowflake.connector.cursor._base.get_stream_ptr", return_value=42),
            patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])),
        ):
            cursor.fetch_arrow_all()

        with pytest.raises(ProgrammingError, match="Cannot use row-by-row fetch methods"):
            cursor.fetchone()

    def test_row_then_pandas_raises(self, cursor):
        cursor._iterator = iter([(1,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()

        with pytest.raises(ProgrammingError, match="Cannot use arrow/pandas fetch methods"):
            list(cursor.fetch_pandas_batches())

    def test_pandas_then_row_raises(self, cursor):
        cursor._fetch_mode = FetchMode.ARROW

        with pytest.raises(ProgrammingError, match="Cannot use row-by-row fetch methods"):
            cursor.fetchall()

    def test_fetchall_then_arrow_raises(self, cursor):
        cursor._iterator = iter([(1,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchall()

        with pytest.raises(ProgrammingError, match="Cannot use arrow/pandas fetch methods"):
            cursor.fetch_arrow_all()

    def test_same_mode_is_fine(self, cursor):
        cursor._iterator = iter([(1,), (2,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            cursor.fetchone()

    def test_execute_resets_fetch_mode(self, cursor, mock_connection):
        mock_connection.is_closed.return_value = False
        result = MagicMock()
        result.columns = []
        result.HasField.return_value = False
        result.sql_state = ""
        mock_connection.db_api.statement_execute_query.return_value.result = result

        cursor._fetch_mode = FetchMode.ARROW
        with (
            patch("snowflake.connector._internal.statement_utils.StatementNewRequest"),
            patch("snowflake.connector._internal.statement_utils.StatementSetSqlQueryRequest"),
            patch("snowflake.connector.cursor._base.StatementExecuteQueryRequest"),
            patch("snowflake.connector._internal.statement_utils.StatementReleaseRequest"),
        ):
            cursor.execute("SELECT 1")

        assert cursor._fetch_mode is None


class TestReset:
    """Unit tests for Cursor.reset method."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_reset_clears_all_state_together(self, cursor):
        """reset() frees heavy result data but preserves lightweight metadata."""
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,)])
        cursor._binding_data = b"data"
        cursor._rownumber = 10
        mock_desc = [MagicMock()]
        cursor._description = mock_desc
        cursor._sqlstate = "42601"
        cursor._sfqid = "abc-123"
        cursor._query = "SELECT 1"
        cursor._fetch_mode = FetchMode.ROW
        cursor._rowcount = 100

        cursor.reset()

        # Cleared by reset
        assert cursor._execute_result is None
        assert cursor._iterator is None
        assert cursor._binding_data is None
        assert cursor._fetch_mode is None
        assert cursor._rowcount is None
        # Preserved by reset (matches old driver)
        assert cursor._rownumber == 10
        assert cursor._description is mock_desc
        assert cursor._sqlstate == "42601"
        assert cursor._sfqid == "abc-123"
        assert cursor._query == "SELECT 1"

    def test_reset_is_idempotent(self, cursor):
        """Calling reset() twice produces the same state as calling it once."""
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,)])
        cursor._fetch_mode = FetchMode.ROW
        cursor._rowcount = 42

        cursor.reset()
        cursor.reset()

        assert cursor._execute_result is None
        assert cursor._iterator is None
        assert cursor._fetch_mode is None
        assert cursor._rowcount is None
        assert cursor._rownumber == -1

    def test_reset_on_fresh_cursor_is_noop(self, cursor):
        """reset() on a freshly created cursor doesn't break anything."""
        cursor.reset()

        assert cursor._execute_result is None
        assert cursor._iterator is None
        assert cursor._sqlstate is None
        assert cursor._fetch_mode is None
        assert cursor._binding_data is None
        assert cursor._rownumber == -1
        assert cursor._rowcount is None

    def test_reset_closing_true_clears_everything_except_rowcount(self, cursor):
        """reset(closing=True) preserves _rowcount in addition to the usual preserved fields."""
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,)])
        cursor._binding_data = b"data"
        cursor._rownumber = 10
        mock_desc = [MagicMock()]
        cursor._description = mock_desc
        cursor._sqlstate = "42601"
        cursor._sfqid = "abc-123"
        cursor._query = "SELECT 1"
        cursor._fetch_mode = FetchMode.ARROW
        cursor._rowcount = 100

        cursor.reset(closing=True)

        # Cleared by reset
        assert cursor._execute_result is None
        assert cursor._iterator is None
        assert cursor._binding_data is None
        assert cursor._fetch_mode is None
        # Preserved by reset (always)
        assert cursor._rownumber == 10
        assert cursor._description is mock_desc
        assert cursor._sqlstate == "42601"
        assert cursor._sfqid == "abc-123"
        assert cursor._query == "SELECT 1"
        # Preserved by reset(closing=True) specifically
        assert cursor._rowcount == 100

    def test_reset_preserves_query_and_sfqid(self, cursor):
        """After reset(), query and sfqid are preserved (matches old driver)."""
        cursor._sfqid = "abc-123"
        cursor._query = "SELECT 1"

        assert cursor.query == "SELECT 1"
        assert cursor.sfqid == "abc-123"

        cursor.reset()

        assert cursor.query == "SELECT 1"
        assert cursor.sfqid == "abc-123"


class TestClose:
    """Unit tests for Cursor.close method."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_close_returns_true_on_success(self, cursor):
        """close() returns True when the cursor was open and is now closed."""
        assert cursor.close() is True

    def test_close_returns_false_when_already_closed(self, cursor):
        """close() returns False when the cursor was already closed."""
        cursor.close()
        assert cursor.close() is False

    def test_close_sets_closed_flag(self, cursor):
        """close() sets _closed to True."""
        cursor.close()
        assert cursor._closed is True

    def test_close_clears_messages(self, cursor):
        """close() empties the messages list."""
        cursor._messages.append((ProgrammingError, {"msg": "test"}))
        cursor.close()
        assert cursor._messages == []

    def test_close_preserves_rowcount(self, cursor):
        """close() preserves _rowcount via reset(closing=True)."""
        cursor._rowcount = 42
        cursor.close()
        assert cursor._rowcount == 42

    def test_close_clears_result_state(self, cursor):
        """close() clears result-related state via reset (except description)."""
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,)])
        mock_desc = [MagicMock()]
        cursor._description = mock_desc
        cursor._fetch_mode = FetchMode.ROW

        cursor.close()

        assert cursor._execute_result is None
        assert cursor._iterator is None
        assert cursor._description is mock_desc
        assert cursor._fetch_mode is None

    def test_close_returns_none_on_exception(self, cursor):
        """close() returns None when reset() raises an exception."""
        with patch.object(cursor, "reset", side_effect=RuntimeError("boom")):
            assert cursor.close() is None

    def test_close_exception_leaves_cursor_unclosed(self, cursor):
        """When close() fails, the cursor stays open so the caller can retry or clean up."""
        original_conn = cursor._connection
        with patch.object(cursor, "reset", side_effect=RuntimeError("boom")):
            cursor.close()

        assert cursor._closed is False
        assert cursor._connection is original_conn

    def test_close_via_context_manager(self, mock_connection):
        """Exiting a context manager calls close()."""
        with SnowflakeCursor(mock_connection) as cur:
            assert not cur._closed
        assert cur._closed is True


class TestResetIntegration:
    """Integration tests for reset() with other cursor methods."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.db_api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        execute_result = MagicMock()
        execute_result.columns = []
        execute_result.HasField = MagicMock(return_value=False)
        execute_result.sql_state = "00000"
        conn.db_api.statement_execute_query.return_value.result = execute_result
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_close_calls_reset_with_closing_true(self, cursor):
        """close() calls reset(closing=True) to preserve rowcount."""
        cursor._rowcount = 42
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,)])

        cursor.close()

        # Rowcount should be preserved
        assert cursor._rowcount == 42
        # Other state should be cleared
        assert cursor._execute_result is None
        assert cursor._iterator is None
        assert cursor._closed is True

    def test_execute_calls_reset_before_executing(self, cursor, mock_connection):
        """execute() calls reset() before executing to clear old state."""
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,)])
        cursor._description = [MagicMock()]
        cursor._rowcount = 100

        cursor.execute("SELECT 1")

        # Old state should have been cleared by reset()
        # New execute_result will be set by the execution
        assert cursor._iterator is None
        assert cursor._binding_data is None

    def test_executemany_calls_reset_once_before_loop(self, cursor, mock_connection):
        """executemany() calls reset() once before the loop, not for each execute."""
        mock_connection.paramstyle = ParamStyle.PYFORMAT
        cursor._rowcount = 100

        with patch.object(cursor, "reset") as mock_reset:
            with patch.object(cursor, "_execute") as mock_execute:
                mock_execute.return_value = cursor
                cursor._rowcount = 1  # simulate execute setting rowcount
                cursor.executemany("INSERT INTO t VALUES (%s)", [(1,), (2,), (3,)])

        # reset should be called once, not 3 times
        mock_reset.assert_called_once()
        # _execute should be called 3 times
        assert mock_execute.call_count == 3

    def test_execute_resets_fetch_mode_allowing_mode_switch(self, cursor, mock_connection):
        """After execute(), the fetch mode is cleared so a different fetch strategy can be used."""
        cursor._fetch_mode = FetchMode.ARROW

        cursor.execute("SELECT 1")

        assert cursor._fetch_mode is None

    def test_execute_overwrites_sqlstate_with_new_result(self, cursor, mock_connection):
        """execute() overwrites sqlstate from the new query result."""
        cursor._sqlstate = "42601"

        cursor.execute("SELECT 1")

        assert cursor._sqlstate is None

    def test_execute_resets_description_before_new_query(self, cursor, mock_connection):
        """execute() clears old description; new one is populated from the result."""
        cursor._description = [MagicMock()]

        cursor.execute("SELECT 1")

        # Mock has no columns, so description becomes None
        assert cursor._description is None

    def test_executemany_server_side_binding_delegates_reset_to_execute(self, cursor, mock_connection):
        """executemany() with server-side (qmark) binding delegates to execute(), which performs its own reset."""
        mock_connection.paramstyle = ParamStyle.QMARK
        cursor._fetch_mode = FetchMode.ARROW
        cursor._sqlstate = "42601"

        cursor.executemany("INSERT INTO t VALUES (?)", [(1,), (2,), (3,)])

        assert cursor._fetch_mode is None
        assert cursor._sqlstate is None

    def test_executemany_empty_params_does_not_reset(self, cursor, mock_connection):
        """executemany() with empty seq_of_parameters returns early without calling reset."""
        cursor._fetch_mode = FetchMode.ARROW
        cursor._rowcount = 42
        cursor._execute_result = MagicMock()

        cursor.executemany("INSERT INTO t VALUES (?)", [])

        assert cursor._fetch_mode == FetchMode.ARROW
        assert cursor._rowcount == 42
        assert cursor._execute_result is not None


class TestDescribe:
    """Unit tests for Cursor.describe method."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.db_api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def _setup_prepare(self, mock_connection, columns=None, query_id="", query="", sql_state=None):
        result = MagicMock()
        result.columns = columns or []
        result.stream.value = (42).to_bytes(8, byteorder="little", signed=False)
        result.query_id = query_id
        result.query = query
        result.sql_state = sql_state
        mock_connection.db_api.statement_prepare.return_value.result = result
        return result

    def test_describe_returns_column_metadata(self, cursor, mock_connection):
        """describe() returns ResultMetadata and updates cursor.description."""
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = "COL1"
        col.HasField = lambda f: f in ("precision", "scale")
        self._setup_prepare(mock_connection, columns=[col])

        with patch("snowflake.connector.cursor._base.release_arrow_stream"):
            result = cursor.describe("SELECT 1 AS COL1")

        assert result is not None
        assert len(result) == 1
        assert result[0].name == "COL1"
        assert cursor.description == result

    def test_describe_returns_none_for_no_columns(self, cursor, mock_connection):
        """describe() returns None when the statement produces no result set."""
        self._setup_prepare(mock_connection, columns=[])

        with patch("snowflake.connector.cursor._base.release_arrow_stream"):
            assert cursor.describe("INSERT INTO t VALUES (1)") is None

    def test_describe_side_effects_with_columns(self, cursor, mock_connection):
        """describe() sets sfqid, query, sqlstate, rowcount when result has columns."""
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = "COL1"  # `name` is reserved by MagicMock; must be set after init
        col.HasField = lambda f: f in ("precision", "scale")
        self._setup_prepare(
            mock_connection,
            columns=[col],
            query_id="01abc-def",
            query="SELECT 1",
            sql_state="00000",
        )

        cursor._rowcount = 42
        cursor._fetch_mode = FetchMode.ARROW

        with patch("snowflake.connector.cursor._base.release_arrow_stream"):
            cursor.describe("SELECT 1")

        assert cursor._execute_result is None
        assert cursor.sfqid == "01abc-def"
        assert cursor.query == "SELECT 1"
        assert cursor.sqlstate is None  # "00000" is normalized to None
        assert cursor.rowcount == 0
        assert cursor._fetch_mode is None

    def test_describe_forwards_non_success_sqlstate(self, cursor, mock_connection):
        """describe() forwards sqlstate when it differs from '00000'."""
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = "COL1"  # `name` is reserved by MagicMock; must be set after init
        col.HasField = lambda f: f in ("precision", "scale")
        self._setup_prepare(
            mock_connection,
            columns=[col],
            sql_state="02000",
        )

        with patch("snowflake.connector.cursor._base.release_arrow_stream"):
            cursor.describe("SELECT 1")

        assert cursor.sqlstate == "02000"

    def test_describe_side_effects_without_columns(self, cursor, mock_connection):
        """describe() resets state; sfqid/query/sqlstate are set from result even without columns."""
        cursor._rowcount = 42
        cursor._fetch_mode = FetchMode.ARROW
        self._setup_prepare(mock_connection)

        with patch("snowflake.connector.cursor._base.release_arrow_stream"):
            cursor.describe("SELECT 1")

        assert cursor._execute_result is None
        assert cursor.sfqid is None
        assert cursor.rowcount is None
        assert cursor._fetch_mode is None

    def test_describe_releases_handle_and_stream(self, cursor, mock_connection):
        """describe() allocates/releases statement handle and releases the arrow stream."""
        self._setup_prepare(mock_connection)

        with patch("snowflake.connector.cursor._base.release_arrow_stream") as mock_iter:
            cursor.describe("SELECT 1")

        mock_connection.db_api.statement_new.assert_called_once()
        mock_connection.db_api.statement_release.assert_called_once()
        mock_iter.assert_called_once()

    def test_describe_raises_when_closed(self, cursor, mock_connection):
        """describe() raises InterfaceError on closed cursor or connection."""
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.describe("SELECT 1")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.describe("SELECT 1")

    def test_describe_propagates_prepare_error(self, cursor, mock_connection):
        """describe() propagates ProgrammingError and captures sqlstate."""
        mock_connection.db_api.statement_prepare.side_effect = ProgrammingError("syntax error", sqlstate="42601")

        with pytest.raises(ProgrammingError):
            cursor.describe("INVALID SQL")

        assert cursor.sqlstate == "42601"


class TestQueryResult:
    """Unit tests for Cursor.query_result method."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def _stub_result(self, mock_connection, **overrides):
        """Set up the mock RPC to return a result with the given overrides."""
        result = MagicMock()
        result.columns = overrides.get("columns", [])
        result.rows_affected = overrides.get("rows_affected", 0)
        result.HasField = MagicMock(return_value=overrides.get("has_rows_affected", False))
        result.sql_state = overrides.get("sql_state", "")
        mock_connection.db_api.connection_get_query_result.return_value.result = result
        return result

    def test_query_result_populates_cursor_state(self, cursor, mock_connection):
        """query_result returns self, sends correct RPC args, and populates all cursor fields."""
        col = MagicMock()
        col.name = "ID"
        col.HasField = MagicMock(return_value=False)
        col.nullable = True

        result = self._stub_result(
            mock_connection,
            columns=[col],
            rows_affected=42,
            has_rows_affected=True,
            sql_state="02000",
        )

        ret = cursor.query_result("01234567-abcd-ef01-0000-000000000001")

        assert ret is cursor
        assert cursor._execute_result is result
        assert cursor.description is not None
        assert len(cursor.description) == 1
        assert cursor.description[0].name == "ID"
        assert cursor.rowcount == 42
        assert cursor.sqlstate == "02000"

        request = mock_connection.db_api.connection_get_query_result.call_args.args[0]
        assert request.conn_handle == ConnectionHandle(id=1)
        assert request.query_id == "01234567-abcd-ef01-0000-000000000001"

    def test_query_result_resets_prior_state(self, cursor, mock_connection):
        """query_result clears iterator and fetch mode from a previous execute."""
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,)])
        cursor._fetch_mode = FetchMode.ROW

        self._stub_result(mock_connection)
        cursor.query_result("qid")

        assert cursor._fetch_mode is None
        assert cursor._iterator is None

    def test_query_result_raises_on_closed_cursor_or_connection(self, cursor, mock_connection):
        """query_result raises InterfaceError when cursor or connection is closed."""
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.query_result("qid")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.query_result("qid")

    def test_query_result_propagates_rpc_error(self, cursor, mock_connection):
        """query_result propagates ProgrammingError from the RPC layer."""
        mock_connection.db_api.connection_get_query_result.side_effect = ProgrammingError("Query has expired")
        with pytest.raises(ProgrammingError, match="Query has expired"):
            cursor.query_result("expired-qid")


class TestQueryResultWaiter:
    """Unit tests for QueryResultWaiter."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.is_still_running = MagicMock(side_effect=lambda s: s in (QueryStatus.RUNNING, QueryStatus.NO_DATA))
        return conn

    def test_returns_immediately_when_query_already_done(self, mock_connection):
        """wait() returns without sleeping when query status is SUCCESS."""
        mock_connection.get_query_status_throw_if_error.return_value = QueryStatus.SUCCESS
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep") as mock_sleep:
            waiter.wait()

        mock_sleep.assert_not_called()

    def test_polls_until_success(self, mock_connection):
        """wait() polls with backoff until the query finishes."""
        mock_connection.get_query_status_throw_if_error.side_effect = [
            QueryStatus.RUNNING,
            QueryStatus.RUNNING,
            QueryStatus.SUCCESS,
        ]
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep") as mock_sleep:
            waiter.wait()

        assert mock_connection.get_query_status_throw_if_error.call_count == 3
        assert mock_sleep.call_count == 2

    def test_raises_on_error_status(self, mock_connection):
        """wait() propagates ProgrammingError from get_query_status_throw_if_error."""
        mock_connection.get_query_status_throw_if_error.side_effect = ProgrammingError("Query failed")
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep"):
            with pytest.raises(ProgrammingError, match="Query failed"):
                waiter.wait()

    def test_raises_after_no_data_max_retry(self, mock_connection):
        """wait() raises DatabaseError after too many NO_DATA responses."""
        mock_connection.get_query_status_throw_if_error.return_value = QueryStatus.NO_DATA
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep"):
            with pytest.raises(DatabaseError, match="Cannot retrieve data"):
                waiter.wait()


class TestGetResultsFromSfqid:
    """Unit tests for Cursor.get_results_from_sfqid."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.get_query_status_throw_if_error.return_value = QueryStatus.SUCCESS
        conn.is_still_running.return_value = False
        result = MagicMock()
        result.columns = []
        result.HasField = MagicMock(return_value=False)
        result.sql_state = ""
        conn.db_api.connection_get_query_result.return_value.result = result
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sets_sfqid_eagerly(self, cursor):
        """get_results_from_sfqid sets sfqid immediately, before any fetch."""
        cursor.get_results_from_sfqid("test-qid")

        assert cursor.sfqid == "test-qid"

    def test_installs_prefetch_hook(self, cursor):
        """get_results_from_sfqid installs a prefetch hook that fires on fetch."""
        cursor.get_results_from_sfqid("test-qid")

        assert cursor._prefetch_hook is not None

    def test_prefetch_hook_fires_on_fetch(self, cursor, mock_connection):
        """First fetch triggers the hook, which calls query_result."""
        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep"):
            cursor.get_results_from_sfqid("test-qid")

        assert cursor._prefetch_hook is not None
        with patch.object(cursor, "query_result") as mock_qr:
            cursor._prefetch_hook()

        mock_qr.assert_called_once_with("test-qid")
        assert cursor._prefetch_hook is None

    def test_raises_on_closed_cursor(self, cursor):
        """get_results_from_sfqid raises InterfaceError when cursor is closed."""
        cursor.close()

        with pytest.raises(InterfaceError):
            cursor.get_results_from_sfqid("qid")

    def test_raises_when_query_already_failed(self, cursor, mock_connection):
        """get_results_from_sfqid raises immediately if status check returns error."""
        mock_connection.get_query_status_throw_if_error.side_effect = ProgrammingError("Query failed")

        with pytest.raises(ProgrammingError, match="Query failed"):
            cursor.get_results_from_sfqid("bad-qid")

    def test_execute_clears_pending_hook(self, cursor, mock_connection):
        """A new execute() cancels a pending prefetch hook."""
        cursor.get_results_from_sfqid("test-qid")
        assert cursor._prefetch_hook is not None

        handle_resp = MagicMock()
        handle_resp.stmt_handle = StatementHandle(id=1)
        mock_connection.db_api.statement_new.return_value = handle_resp
        result = MagicMock()
        result.columns = []
        result.HasField = MagicMock(return_value=False)
        result.sql_state = ""
        mock_connection.db_api.statement_execute_query.return_value.result = result
        with (
            patch("snowflake.connector._internal.statement_utils.StatementNewRequest"),
            patch("snowflake.connector._internal.statement_utils.StatementSetSqlQueryRequest"),
            patch("snowflake.connector.cursor._base.StatementExecuteQueryRequest"),
            patch("snowflake.connector._internal.statement_utils.StatementReleaseRequest"),
        ):
            cursor.execute("SELECT 1")

        assert cursor._prefetch_hook is None
