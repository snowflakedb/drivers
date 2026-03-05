"""
Unit tests for PEP 249 Cursor class.
"""

from decimal import Decimal
from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    StatementHandle,
)
from snowflake.connector.cursor import SnowflakeCursor, SnowflakeCursorBase
from snowflake.connector.errors import ProgrammingError


class TestFetchone:
    """Unit tests for Cursor.fetchone method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchone_returns_single_row(self, cursor):
        """Test fetchone returns a single row tuple."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result == (1,)

    def test_fetchone_returns_none_when_exhausted(self, cursor):
        """Test fetchone returns None when no more rows."""
        mock_iterator = iter([])
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result is None

    def test_fetchone_sequential_calls(self, cursor):
        """Test sequential fetchone calls return rows in order."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            first = cursor.fetchone()
            second = cursor.fetchone()
            third = cursor.fetchone()
            fourth = cursor.fetchone()

        assert first == (1,)
        assert second == (2,)
        assert third == (3,)
        assert fourth is None

    def test_fetchone_calls_get_iterator_if_iterator_is_none(self, cursor):
        """Test fetchone calls _get_iterator."""
        mock_ensure = MagicMock(return_value=iter([(1,)]))

        with patch.object(cursor, "_get_iterator", mock_ensure):
            cursor.fetchone()

        mock_ensure.assert_called_once()

    def test_fetchone_with_multi_column_row(self, cursor):
        """Test fetchone with multiple columns."""
        mock_rows = [(1, "hello", 3.14)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result == (1, "hello", 3.14)

    def test_fetchone_preserves_types(self, cursor):
        """Test fetchone preserves data types."""
        mock_rows = [(1, "text", Decimal("3.14"), None, True)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
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

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result == ()

    def test_fetchone_after_exhaustion_returns_none(self, cursor):
        """Test fetchone consistently returns None after exhaustion."""
        mock_rows = [(1,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchone()  # Consume the row
            result1 = cursor.fetchone()
            result2 = cursor.fetchone()

        assert result1 is None
        assert result2 is None


class TestFetchall:
    """Unit tests for Cursor.fetchall method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchall_returns_all_rows(self, cursor):
        """Test fetchall returns all rows as a list."""
        mock_rows = [(1,), (2,), (3,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result == [(1,), (2,), (3,)]

    def test_fetchall_returns_empty_list_when_no_rows(self, cursor):
        """Test fetchall returns empty list when no rows."""
        cursor._iterator = iter([])

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_calls_get_iterator_if_iterator_is_none(self, cursor):
        """Test fetchall calls _get_iterator."""
        mock_ensure = MagicMock()

        with patch.object(cursor, "_get_iterator", mock_ensure):
            cursor.fetchall()

        mock_ensure.assert_called_once()

    def test_fetchall_with_single_row(self, cursor):
        """Test fetchall with single row."""
        mock_rows = [(42,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
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

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result == [(1, "a", 1.0), (2, "b", 2.0), (3, "c", 3.0)]

    def test_fetchall_preserves_types(self, cursor):
        """Test fetchall preserves data types in rows."""
        mock_rows = [
            (1, "text", Decimal("3.14"), None),
            (2, "more", Decimal("2.71"), True),
        ]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
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

        with patch.object(cursor, "_get_iterator"):
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

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchall()  # Consume all rows
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_with_large_result_set(self, cursor):
        """Test fetchall with large number of rows."""
        mock_rows = [(i,) for i in range(1000)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert len(result) == 1000
        assert result[0] == (0,)
        assert result[999] == (999,)

    def test_fetchall_returns_list_not_iterator(self, cursor):
        """Test fetchall returns a list, not an iterator."""
        mock_rows = [(1,), (2,), (3,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert isinstance(result, list)


class TestFetchmany:
    """Unit tests for Cursor.fetchmany method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

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


class TestFetchmanyArraysizeAttribute:
    """Tests for arraysize attribute interaction with fetchmany."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

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
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_rownumber_none_before_fetch(self, cursor):
        """rownumber is None before any rows have been fetched."""
        assert cursor.rownumber is None

    def test_rownumber_increments_with_fetchone(self, cursor):
        """rownumber increments by 1 for each fetchone call."""
        cursor._iterator = iter([(1,), (2,), (3,)])

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0
            cursor.fetchone()
            assert cursor.rownumber == 1
            cursor.fetchone()
            assert cursor.rownumber == 2

    def test_rownumber_stays_after_fetchone_exhausted(self, cursor):
        """rownumber stays at last value when fetchone returns None."""
        cursor._iterator = iter([(1,)])

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0
            cursor.fetchone()  # returns None
            assert cursor.rownumber == 0

    def test_rownumber_updated_by_fetchall(self, cursor):
        """rownumber reflects total rows fetched after fetchall."""
        cursor._iterator = iter([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchall()
            assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchall_after_partial_fetchone(self, cursor):
        """rownumber is correct when fetchall follows partial fetchone consumption."""
        cursor._iterator = iter([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchone()
            cursor.fetchone()
            assert cursor.rownumber == 1
            cursor.fetchall()
            assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchmany(self, cursor):
        """rownumber increments correctly through fetchmany calls."""
        cursor._iterator = iter([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchmany(3)
            assert cursor.rownumber == 2
            cursor.fetchmany(2)
            assert cursor.rownumber == 4

    def test_rownumber_fetchall_on_empty_result(self, cursor):
        """rownumber stays None when fetchall returns no rows."""
        cursor._iterator = iter([])

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchall()
            assert cursor.rownumber is None

    def test_rownumber_none_after_execute_resets(self, cursor):
        """rownumber resets to None after a new execute call."""
        cursor._iterator = iter([(1,), (2,)])

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0

        cursor._rownumber = -1  # simulates what execute() does
        assert cursor.rownumber is None
