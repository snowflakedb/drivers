"""
Unit tests for PEP 249 Cursor class.

Tests mock at the :class:`BlockingImmutableCursor` boundary — the cursor
never touches ``core_driver`` or the FFI layer. This matches production
code where every RPC goes through ``ImmutableCursor``.
"""

from decimal import Decimal
from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.binding_converters import ParamStyle
from snowflake.connector._internal.errorcode import ER_NO_PYARROW
from snowflake.connector._internal.extras import (
    MissingOptionalDependency,
)
from snowflake.connector._internal.extras import (
    check_dependency as _real_check_dependency,
)
from snowflake.connector.constants import QueryStatus
from snowflake.connector.cursor import QueryResultStats, SnowflakeCursor, SnowflakeCursorBase
from snowflake.connector.cursor._blocking_immutable_cursor import BlockingImmutableCursor
from snowflake.connector.cursor._query_result import _QueryResult
from snowflake.connector.cursor._query_result_waiter import QueryResultWaiter
from snowflake.connector.errors import DatabaseError, InterfaceError, ProgrammingError


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_mock_immutable(
    rows=None,
    *,
    description=None,
    rowcount=None,
    sfqid=None,
    query=None,
    sqlstate=None,
    stats=None,
    multi_query_ids=None,
):
    """Build a mock :class:`BlockingImmutableCursor` with canned fetch behaviour.

    *rows* is a list of tuples/dicts. Fetch methods consume from this list in
    order, tracking ``rownumber`` the same way the real cursor does.
    """
    mock = MagicMock(spec=BlockingImmutableCursor)

    qr = _QueryResult(
        description=description,
        rowcount=rowcount,
        sfqid=sfqid,
        query=query,
        sqlstate=sqlstate,
        stats=stats,
    )
    mock.query_result = qr
    mock.multi_query_ids = multi_query_ids

    if rows is None:
        rows = []
    pos = [0]
    rownumber = [-1]

    def _fetchone():
        if pos[0] >= len(rows):
            return None
        row = rows[pos[0]]
        pos[0] += 1
        rownumber[0] = pos[0] - 1
        mock.rownumber = rownumber[0]
        return row

    def _fetchmany(size=None):
        if size is None:
            size = 1
        batch = rows[pos[0] : pos[0] + size]
        pos[0] += len(batch)
        if batch:
            rownumber[0] = pos[0] - 1
            mock.rownumber = rownumber[0]
        return batch

    def _fetchall():
        batch = rows[pos[0] :]
        pos[0] = len(rows)
        if batch:
            rownumber[0] = pos[0] - 1
            mock.rownumber = rownumber[0]
        return batch

    mock.fetchone.side_effect = _fetchone
    mock.fetchmany.side_effect = _fetchmany
    mock.fetchall.side_effect = _fetchall
    mock.rownumber = rownumber[0]
    mock.description = description
    mock.rowcount = rowcount
    mock.sfqid = sfqid
    mock.query = query
    mock.sqlstate = sqlstate
    mock.stats = stats if stats is not None else QueryResultStats()

    return mock


def _inject_immutable(cursor, mock_immutable):
    """Simulate what ``execute`` does: adopt the mock immutable cursor."""
    cursor._immutable = mock_immutable
    cursor._query_result = mock_immutable.query_result


# ---------------------------------------------------------------------------
# Fetch tests
# ---------------------------------------------------------------------------


class TestFetchone:
    """Unit tests for Cursor.fetchone method."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_fetchone_returns_single_row(self, cursor):
        """Test fetchone returns a single row tuple."""
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert cursor.fetchone() == (1,)

    def test_fetchone_returns_none_when_exhausted(self, cursor):
        """Test fetchone returns None when no more rows."""
        _inject_immutable(cursor, _make_mock_immutable([]))
        assert cursor.fetchone() is None

    def test_fetchone_sequential_calls(self, cursor):
        """Test sequential fetchone calls return rows in order."""
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert cursor.fetchone() == (1,)
        assert cursor.fetchone() == (2,)
        assert cursor.fetchone() == (3,)
        assert cursor.fetchone() is None

    def test_fetchone_with_multi_column_row(self, cursor):
        """Test fetchone with multiple columns."""
        _inject_immutable(cursor, _make_mock_immutable([(1, "hello", 3.14)]))
        assert cursor.fetchone() == (1, "hello", 3.14)

    def test_fetchone_preserves_types(self, cursor):
        """Test fetchone preserves data types."""
        _inject_immutable(cursor, _make_mock_immutable([(1, "text", Decimal("3.14"), None, True)]))
        result = cursor.fetchone()
        assert result[0] == 1
        assert result[1] == "text"
        assert result[2] == Decimal("3.14")
        assert isinstance(result[2], Decimal)
        assert result[3] is None
        assert result[4] is True

    def test_fetchone_with_empty_tuple_row(self, cursor):
        """Test fetchone handles empty tuple row."""
        _inject_immutable(cursor, _make_mock_immutable([()]))
        assert cursor.fetchone() == ()

    def test_fetchone_after_exhaustion_returns_none(self, cursor):
        """Test fetchone consistently returns None after exhaustion."""
        _inject_immutable(cursor, _make_mock_immutable([(1,)]))
        cursor.fetchone()
        assert cursor.fetchone() is None
        assert cursor.fetchone() is None


class TestFetchall:
    """Unit tests for Cursor.fetchall method."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_fetchall_returns_all_rows(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert cursor.fetchall() == [(1,), (2,), (3,)]

    def test_fetchall_returns_empty_list_when_no_rows(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([]))
        assert cursor.fetchall() == []

    def test_fetchall_with_single_row(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(42,)]))
        result = cursor.fetchall()
        assert result == [(42,)]
        assert len(result) == 1

    def test_fetchall_with_multi_column_rows(self, cursor):
        _inject_immutable(
            cursor,
            _make_mock_immutable(
                [
                    (1, "a", 1.0),
                    (2, "b", 2.0),
                    (3, "c", 3.0),
                ]
            ),
        )
        assert cursor.fetchall() == [(1, "a", 1.0), (2, "b", 2.0), (3, "c", 3.0)]

    def test_fetchall_preserves_types(self, cursor):
        _inject_immutable(
            cursor,
            _make_mock_immutable(
                [
                    (1, "text", Decimal("3.14"), None),
                    (2, "more", Decimal("2.71"), True),
                ]
            ),
        )
        result = cursor.fetchall()
        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert isinstance(result[1][2], Decimal)

    def test_fetchall_after_partial_fetchone(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        cursor.fetchone()
        cursor.fetchone()
        assert cursor.fetchall() == [(3,), (4,), (5,)]

    def test_fetchall_returns_empty_after_exhaustion(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,)]))
        cursor.fetchall()
        assert cursor.fetchall() == []

    def test_fetchall_with_large_result_set(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(i,) for i in range(1000)]))
        result = cursor.fetchall()
        assert len(result) == 1000
        assert result[0] == (0,)
        assert result[999] == (999,)

    def test_fetchall_returns_list_not_iterator(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert isinstance(cursor.fetchall(), list)

    def test_fetchall_returns_empty_without_execute(self, cursor):
        """fetchall returns empty list when no query has been executed."""
        assert cursor.fetchall() == []


class TestFetchmany:
    """Unit tests for Cursor.fetchmany method."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_fetchmany_default_uses_arraysize(self, cursor):
        cursor.arraysize = 3
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        assert cursor.fetchmany() == [(1,), (2,), (3,)]

    def test_fetchmany_with_explicit_size(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        assert cursor.fetchmany(2) == [(1,), (2,)]

    def test_fetchmany_returns_fewer_rows_when_exhausted(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,)]))
        assert cursor.fetchmany(5) == [(1,), (2,)]

    def test_fetchmany_returns_empty_list_when_no_rows(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([]))
        assert cursor.fetchmany(5) == []

    def test_fetchmany_with_size_zero(self, cursor):
        result = cursor.fetchmany(0)
        assert result == []

    def test_fetchmany_with_negative_size_raises_error(self, cursor):
        with pytest.raises(ProgrammingError, match="The number of rows is not zero or positive number: -1"):
            cursor.fetchmany(-1)

    def test_fetchmany_with_negative_size_various_values(self, cursor):
        with pytest.raises(ProgrammingError, match="The number of rows is not zero or positive number: -42"):
            cursor.fetchmany(-42)

    def test_fetchmany_sequential_calls(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        assert cursor.fetchmany(2) == [(1,), (2,)]
        assert cursor.fetchmany(2) == [(3,), (4,)]
        assert cursor.fetchmany(2) == [(5,)]

    def test_fetchmany_after_exhausted_returns_empty(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,)]))
        cursor.fetchmany(5)
        assert cursor.fetchmany(5) == []

    def test_fetchmany_respects_changed_arraysize(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,), (6,), (7,), (8,)]))
        cursor.arraysize = 2
        assert cursor.fetchmany() == [(1,), (2,)]
        cursor.arraysize = 4
        assert cursor.fetchmany() == [(3,), (4,), (5,), (6,)]

    def test_fetchmany_with_size_one(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert cursor.fetchmany(1) == [(1,)]

    def test_fetchmany_with_large_size(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(i,) for i in range(10)]))
        assert cursor.fetchmany(1000) == [(i,) for i in range(10)]

    def test_fetchmany_default_arraysize_is_one(self, cursor):
        assert cursor.arraysize == 1
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert cursor.fetchmany() == [(1,)]

    def test_fetchmany_with_multi_column_rows(self, cursor):
        _inject_immutable(
            cursor,
            _make_mock_immutable(
                [
                    (1, "a", 1.0),
                    (2, "b", 2.0),
                    (3, "c", 3.0),
                ]
            ),
        )
        assert cursor.fetchmany(2) == [(1, "a", 1.0), (2, "b", 2.0)]

    def test_fetchmany_preserves_row_types(self, cursor):
        _inject_immutable(
            cursor,
            _make_mock_immutable(
                [
                    (1, "text", Decimal("3.14"), None),
                    (2, "more", Decimal("2.71"), True),
                ]
            ),
        )
        result = cursor.fetchmany(2)
        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert result[0][3] is None
        assert result[1][3] is True

    def test_fetchmany_after_partial_fetchone(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        cursor.fetchone()
        cursor.fetchone()
        assert cursor.fetchmany(2) == [(3,), (4,)]

    def test_fetchmany_returns_empty_without_execute(self, cursor):
        """fetchmany returns empty list when no query has been executed."""
        assert cursor.fetchmany() == []


# ---------------------------------------------------------------------------
# Handle / lifecycle tests
# ---------------------------------------------------------------------------


class TestHandleLifecycle:
    """Tests that ImmutableCursor handles are properly managed through execute/reset/close.

    Replaces the old core_driver-level handle tests. Now we verify that
    the cursor creates and closes :class:`BlockingImmutableCursor` instances
    at the right points.
    """

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.config.numpy = False
        conn.paramstyle = ParamStyle.PYFORMAT
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def _mock_execute(self, **qr_kwargs):
        """Return a context manager that patches BlockingImmutableCursor.execute."""
        immutable = _make_mock_immutable(**qr_kwargs)
        return patch.object(BlockingImmutableCursor, "execute", return_value=immutable), immutable

    def test_execute_creates_immutable(self, cursor):
        patcher, immutable = self._mock_execute()
        with patcher:
            cursor.execute("SELECT 1")
        assert cursor._immutable is immutable

    def test_reset_closes_immutable(self, cursor):
        patcher, immutable = self._mock_execute()
        with patcher:
            cursor.execute("SELECT 1")
        cursor.reset()
        immutable.close.assert_called_once()
        assert cursor._immutable is None

    def test_close_closes_immutable(self, cursor):
        patcher, immutable = self._mock_execute()
        with patcher:
            cursor.execute("SELECT 1")
        cursor.close()
        immutable.close.assert_called_once()

    def test_sequential_executes_close_previous(self, cursor):
        immutables = []

        def _make(*args, **kwargs):
            m = _make_mock_immutable()
            immutables.append(m)
            return m

        with patch.object(BlockingImmutableCursor, "execute", side_effect=_make):
            for i in range(5):
                cursor.execute(f"SELECT {i}")

        # First 4 should have been closed by the reset at the start of each subsequent execute.
        for m in immutables[:-1]:
            m.close.assert_called_once()
        # Last one is still alive.
        immutables[-1].close.assert_not_called()

    def test_close_without_execute_does_not_error(self, cursor):
        cursor.close()


# ---------------------------------------------------------------------------
# Sqlstate / sfqid tests
# ---------------------------------------------------------------------------


class TestSqlstate:
    """Unit tests for Cursor.sqlstate property."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.config.numpy = False
        conn.paramstyle = ParamStyle.PYFORMAT
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sqlstate_none_before_execute(self, cursor):
        assert cursor.sqlstate is None

    def test_sqlstate_none_after_successful_execute(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable(sqlstate=None)):
            cursor.execute("SELECT 1")
        assert cursor.sqlstate is None

    def test_sqlstate_populated_with_error_code(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable(sqlstate="42601")):
            cursor.execute("SELECT 1")
        assert cursor.sqlstate == "42601"

    def test_sqlstate_updates_on_subsequent_execute(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable(sqlstate="42601")):
            cursor.execute("SELECT 1")
        assert cursor.sqlstate == "42601"

        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable(sqlstate=None)):
            cursor.execute("SELECT 2")
        assert cursor.sqlstate is None

    def test_sqlstate_set_from_error_on_failed_execute(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute", side_effect=ProgrammingError("error", sqlstate="42601")):
            with pytest.raises(ProgrammingError):
                cursor.execute("INVALID SQL")
        assert cursor.sqlstate == "42601"

    def test_sqlstate_set_to_none_when_error_has_no_sqlstate(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute", side_effect=ProgrammingError("error", sqlstate=None)):
            with pytest.raises(ProgrammingError):
                cursor.execute("INVALID SQL")
        assert cursor.sqlstate is None


class TestSfqidOnFailedQuery:
    """Unit tests for cursor.sfqid propagation when execute raises."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.config.numpy = False
        conn.paramstyle = ParamStyle.PYFORMAT
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sfqid_set_from_error_on_failed_execute(self, cursor):
        with patch.object(
            BlockingImmutableCursor, "execute", side_effect=ProgrammingError("error", sfqid="01abc-def-12345")
        ):
            with pytest.raises(ProgrammingError):
                cursor.execute("INVALID SQL")
        assert cursor.sfqid == "01abc-def-12345"

    def test_sfqid_none_when_error_has_no_sfqid(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute", side_effect=ProgrammingError("error")):
            with pytest.raises(ProgrammingError):
                cursor.execute("INVALID SQL")
        assert cursor.sfqid is None


# ---------------------------------------------------------------------------
# QueryResultStats
# ---------------------------------------------------------------------------


class TestQueryResultStats:
    """Unit tests for QueryResultStats NamedTuple."""

    def test_default_all_none(self):
        stats = QueryResultStats()
        assert stats.num_rows_inserted is None
        assert stats.num_rows_deleted is None
        assert stats.num_rows_updated is None
        assert stats.num_dml_duplicates is None

    def test_positional_construction(self):
        stats = QueryResultStats(10, 20, 30, 5)
        assert stats.num_rows_inserted == 10
        assert stats.num_rows_deleted == 20
        assert stats.num_rows_updated == 30
        assert stats.num_dml_duplicates == 5

    def test_keyword_construction(self):
        stats = QueryResultStats(num_rows_inserted=1, num_rows_updated=2)
        assert stats.num_rows_inserted == 1
        assert stats.num_rows_deleted is None
        assert stats.num_rows_updated == 2
        assert stats.num_dml_duplicates is None

    def test_is_named_tuple(self):
        stats = QueryResultStats(1, 2, 3, 4)
        assert isinstance(stats, tuple)
        assert len(stats) == 4
        assert stats[0] == 1
        assert stats._fields == ("num_rows_inserted", "num_rows_deleted", "num_rows_updated", "num_dml_duplicates")

    def test_equality(self):
        assert QueryResultStats(1, 2, 3, 4) == QueryResultStats(1, 2, 3, 4)

    def test_all_none_equality(self):
        assert QueryResultStats() == QueryResultStats(None, None, None, None)

    def test_from_query_stats_all_fields_present(self):
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 10
        mock_stats.num_rows_deleted = 5
        mock_stats.num_rows_updated = 3
        mock_stats.num_dml_duplicates = 1
        mock_stats.HasField.return_value = True
        assert QueryResultStats.from_query_stats(mock_stats) == QueryResultStats(10, 5, 3, 1)

    def test_from_query_stats_partial_fields(self):
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 42
        mock_stats.HasField.side_effect = lambda name: name == "num_rows_inserted"
        result = QueryResultStats.from_query_stats(mock_stats)
        assert result == QueryResultStats(
            num_rows_inserted=42, num_rows_deleted=None, num_rows_updated=None, num_dml_duplicates=None
        )

    def test_from_query_stats_no_fields_present(self):
        mock_stats = MagicMock()
        mock_stats.HasField.return_value = False
        assert QueryResultStats.from_query_stats(mock_stats) == QueryResultStats()

    def test_from_query_stats_zero_values(self):
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 0
        mock_stats.num_rows_deleted = 0
        mock_stats.num_rows_updated = 0
        mock_stats.num_dml_duplicates = 0
        mock_stats.HasField.return_value = True
        assert QueryResultStats.from_query_stats(mock_stats) == QueryResultStats(0, 0, 0, 0)


# ---------------------------------------------------------------------------
# Stats property
# ---------------------------------------------------------------------------


class TestStats:
    """Unit tests for Cursor.stats property."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_stats_returns_all_none_before_execute(self, cursor):
        assert cursor.stats == QueryResultStats(None, None, None, None)

    def test_stats_returns_all_fields_when_present(self, cursor):
        cursor._query_result.stats = QueryResultStats(10, 5, 3, 1)
        assert cursor.stats == QueryResultStats(10, 5, 3, 1)

    def test_stats_returns_partial_fields(self, cursor):
        cursor._query_result.stats = QueryResultStats(num_rows_inserted=10)
        result = cursor.stats
        assert result.num_rows_inserted == 10
        assert result.num_rows_deleted is None

    def test_stats_distinguishes_zero_from_absent(self, cursor):
        cursor._query_result.stats = QueryResultStats(0, 0, 0, 0)
        assert cursor.stats == QueryResultStats(0, 0, 0, 0)

    def test_stats_returns_query_result_stats_type(self, cursor):
        assert isinstance(cursor.stats, QueryResultStats)
        cursor._query_result.stats = QueryResultStats(1, 2, 3, 4)
        assert isinstance(cursor.stats, QueryResultStats)

    def test_stats_updates_on_subsequent_execute(self, cursor):
        cursor._query_result.stats = QueryResultStats(num_rows_inserted=5)
        assert cursor.stats.num_rows_inserted == 5
        cursor._query_result.stats = QueryResultStats(num_rows_inserted=20)
        assert cursor.stats.num_rows_inserted == 20


# ---------------------------------------------------------------------------
# Arraysize
# ---------------------------------------------------------------------------


class TestFetchmanyArraysizeAttribute:
    """Tests for arraysize attribute interaction with fetchmany."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_arraysize_default(self, cursor):
        assert cursor.arraysize == 1

    def test_arraysize_is_property(self):
        assert isinstance(SnowflakeCursorBase.__dict__["arraysize"], property)

    def test_arraysize_instance_independent(self, cursor):
        assert cursor.arraysize == 1
        cursor.arraysize = 10
        assert cursor.arraysize == 10

    def test_fetchmany_uses_instance_arraysize(self, cursor):
        cursor.arraysize = 5
        _inject_immutable(cursor, _make_mock_immutable([(i,) for i in range(10)]))
        result = cursor.fetchmany()
        assert len(result) == 5


# ---------------------------------------------------------------------------
# Rownumber
# ---------------------------------------------------------------------------


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
        assert cursor.rownumber is None

    def test_rownumber_increments_with_fetchone(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        cursor.fetchone()
        assert cursor.rownumber == 0
        cursor.fetchone()
        assert cursor.rownumber == 1
        cursor.fetchone()
        assert cursor.rownumber == 2

    def test_rownumber_stays_after_fetchone_exhausted(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,)]))
        cursor.fetchone()
        assert cursor.rownumber == 0
        cursor.fetchone()  # returns None
        assert cursor.rownumber == 0

    def test_rownumber_updated_by_fetchall(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        cursor.fetchall()
        assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchall_after_partial_fetchone(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        cursor.fetchone()
        cursor.fetchone()
        assert cursor.rownumber == 1
        cursor.fetchall()
        assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchmany(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        cursor.fetchmany(3)
        assert cursor.rownumber == 2
        cursor.fetchmany(2)
        assert cursor.rownumber == 4

    def test_rownumber_fetchall_on_empty_result(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([]))
        cursor.fetchall()
        assert cursor.rownumber is None

    def test_rownumber_none_after_execute_resets(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,)]))
        cursor.fetchone()
        assert cursor.rownumber == 0
        cursor._rownumber = -1  # simulates what execute() does
        assert cursor.rownumber is None


# ---------------------------------------------------------------------------
# Arrow / Pandas fetch tests
# ---------------------------------------------------------------------------


class TestCheckCanUseArrowResultset:
    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_no_error_when_pyarrow_installed(self, cursor):
        with patch("snowflake.connector.cursor._base.pyarrow", MagicMock()):
            cursor.check_can_use_arrow_resultset()

    def test_raises_programming_error_when_pyarrow_missing(self, cursor):
        with patch("snowflake.connector.cursor._base.pyarrow", MissingOptionalDependency(dep="pyarrow")):
            with pytest.raises(ProgrammingError) as excinfo:
                cursor.check_can_use_arrow_resultset()
            assert excinfo.value.errno == ER_NO_PYARROW

    def test_error_message_contains_install_link(self, cursor):
        with patch("snowflake.connector.cursor._base.pyarrow", MissingOptionalDependency(dep="pyarrow")):
            with pytest.raises(ProgrammingError, match="python-connector-pandas"):
                cursor.check_can_use_arrow_resultset()


class TestCheckCanUsePandas:
    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_no_error_when_pandas_installed(self, cursor):
        with patch("snowflake.connector.cursor._base.pandas", MagicMock()):
            cursor.check_can_use_pandas()

    def test_raises_programming_error_when_pandas_missing(self, cursor):
        with patch("snowflake.connector.cursor._base.pandas", MissingOptionalDependency(dep="pandas")):
            with pytest.raises(ProgrammingError) as excinfo:
                cursor.check_can_use_pandas()
            assert excinfo.value.errno == ER_NO_PYARROW

    def test_error_message_contains_install_link(self, cursor):
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
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([batch1, batch2])):
            tables = list(cursor.fetch_arrow_batches())

        assert tables == [table1, table2]
        self.pa.Table.from_batches.assert_any_call([batch1])
        self.pa.Table.from_batches.assert_any_call([batch2])

    def test_yields_nothing_for_empty_stream(self, cursor):
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])):
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

    def test_passes_force_microsecond_precision(self, cursor, mock_connection):
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])) as mock_get:
            list(cursor.fetch_arrow_batches(force_microsecond_precision=True))

        _, kwargs = mock_get.call_args
        assert kwargs["force_microsecond_precision"] is True


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
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([batch1, batch2])):
            result = cursor.fetch_arrow_all()
        assert result is mock_table

    def test_returns_none_for_empty_stream(self, cursor):
        mock_iterator = MagicMock()
        mock_iterator.__iter__ = MagicMock(return_value=iter([]))
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=mock_iterator):
            result = cursor.fetch_arrow_all()
        assert result is None

    def test_returns_empty_table_with_force_return_table(self, cursor):
        mock_empty_table = MagicMock()
        mock_schema = MagicMock()
        mock_schema.empty_table.return_value = mock_empty_table
        mock_iterator = MagicMock()
        mock_iterator.__iter__ = MagicMock(return_value=iter([]))
        mock_iterator.get_converted_schema.return_value = mock_schema
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=mock_iterator):
            result = cursor.fetch_arrow_all(force_return_table=True)
        assert result is mock_empty_table

    def test_returns_none_without_force_return_table(self, cursor):
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])):
            result = cursor.fetch_arrow_all(force_return_table=False)
        assert result is None

    def test_passes_force_microsecond_precision(self, cursor, mock_connection):
        immutable = _make_mock_immutable()
        immutable.get_arrow_stream_ptr.return_value = 42
        _inject_immutable(cursor, immutable)

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])) as mock_get:
            cursor.fetch_arrow_all(force_microsecond_precision=True)

        _, kwargs = mock_get.call_args
        assert kwargs["force_microsecond_precision"] is True


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
            assert cursor.fetch_pandas_all() is mock_df

    def test_returns_empty_dataframe_for_empty_stream(self, cursor):
        mock_empty_table = MagicMock()
        mock_empty_df = MagicMock()
        mock_empty_table.to_pandas.return_value = mock_empty_df
        with patch.object(cursor, "fetch_arrow_all", return_value=mock_empty_table) as mock_fetch:
            result = cursor.fetch_pandas_all()
        assert result is mock_empty_df
        mock_fetch.assert_called_once_with(force_return_table=True)

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


# ---------------------------------------------------------------------------
# Reset / close
# ---------------------------------------------------------------------------


class TestReset:
    """Unit tests for Cursor.reset method."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_reset_clears_all_state_together(self, cursor):
        mock_desc = [MagicMock()]
        cursor._query_result = _QueryResult(
            description=mock_desc,
            sqlstate="42601",
            sfqid="abc-123",
            query="SELECT 1",
            rowcount=100,
        )
        cursor._binding_data = b"data"
        cursor._rownumber = 10

        cursor.reset()

        assert cursor._binding_data is None
        assert cursor._query_result.rowcount is None
        assert cursor._rownumber == 10
        assert cursor._query_result.description is mock_desc
        assert cursor._query_result.sqlstate == "42601"
        assert cursor._query_result.sfqid == "abc-123"
        assert cursor._query_result.query == "SELECT 1"

    def test_reset_is_idempotent(self, cursor):
        cursor._query_result = _QueryResult(rowcount=42)
        cursor.reset()
        cursor.reset()
        assert cursor._query_result.rowcount is None
        assert cursor._rownumber == -1

    def test_reset_on_fresh_cursor_is_noop(self, cursor):
        cursor.reset()
        assert cursor.sqlstate is None
        assert cursor._binding_data is None
        assert cursor._rownumber == -1
        assert cursor.rowcount is None

    def test_reset_closing_true_clears_everything_except_rowcount(self, cursor):
        mock_desc = [MagicMock()]
        cursor._query_result = _QueryResult(
            description=mock_desc,
            sqlstate="42601",
            sfqid="abc-123",
            query="SELECT 1",
            rowcount=100,
        )
        cursor._binding_data = b"data"
        cursor._rownumber = 10

        cursor.reset(closing=True)

        assert cursor._binding_data is None
        assert cursor._rownumber == 10
        assert cursor._query_result.description is mock_desc
        assert cursor._query_result.sqlstate == "42601"
        assert cursor._query_result.sfqid == "abc-123"
        assert cursor._query_result.query == "SELECT 1"
        assert cursor._query_result.rowcount == 100

    def test_reset_preserves_query_and_sfqid(self, cursor):
        cursor._query_result.sfqid = "abc-123"
        cursor._query_result.query = "SELECT 1"
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
        assert cursor.close() is True

    def test_close_returns_false_when_already_closed(self, cursor):
        cursor.close()
        assert cursor.close() is False

    def test_close_sets_closed_flag(self, cursor):
        cursor.close()
        assert cursor._closed is True

    def test_close_clears_messages(self, cursor):
        cursor._messages.append((ProgrammingError, {"msg": "test"}))
        cursor.close()
        assert cursor._messages == []

    def test_close_preserves_rowcount(self, cursor):
        cursor._query_result.rowcount = 42
        cursor.close()
        assert cursor._query_result.rowcount == 42

    def test_close_clears_result_state(self, cursor):
        mock_desc = [MagicMock()]
        cursor._query_result = _QueryResult(description=mock_desc)
        cursor.close()
        assert cursor._query_result.description is mock_desc

    def test_close_returns_none_on_exception(self, cursor):
        with patch.object(cursor, "reset", side_effect=RuntimeError("boom")):
            assert cursor.close() is None

    def test_close_exception_leaves_cursor_unclosed(self, cursor):
        original_conn = cursor._connection
        with patch.object(cursor, "reset", side_effect=RuntimeError("boom")):
            cursor.close()
        assert cursor._closed is False
        assert cursor._connection is original_conn

    def test_close_via_context_manager(self, mock_connection):
        with SnowflakeCursor(mock_connection) as cur:
            assert not cur._closed
        assert cur._closed is True


class TestResetIntegration:
    """Integration tests for reset() with other cursor methods."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.config.numpy = False
        conn.paramstyle = ParamStyle.PYFORMAT
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_close_calls_reset_with_closing_true(self, cursor):
        cursor._query_result = _QueryResult(rowcount=42)
        cursor.close()
        assert cursor._query_result.rowcount == 42
        assert cursor._closed is True

    def test_execute_calls_reset_before_executing(self, cursor):
        cursor._query_result = _QueryResult(description=[MagicMock()], rowcount=100)
        cursor._binding_data = b"old"

        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable()):
            cursor.execute("SELECT 1")
        assert cursor._binding_data is None

    def test_executemany_calls_reset_once_before_loop(self, cursor):
        cursor._connection.paramstyle = ParamStyle.PYFORMAT
        cursor._query_result.rowcount = 100

        with patch.object(cursor, "reset") as mock_reset:
            with patch.object(cursor, "_execute") as mock_execute:
                mock_execute.return_value = cursor
                cursor._query_result.rowcount = 1
                cursor.executemany("INSERT INTO t VALUES (%s)", [(1,), (2,), (3,)])
        mock_reset.assert_called_once()
        assert mock_execute.call_count == 3

    def test_execute_overwrites_sqlstate_with_new_result(self, cursor):
        cursor._query_result.sqlstate = "42601"
        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable(sqlstate=None)):
            cursor.execute("SELECT 1")
        assert cursor.sqlstate is None

    def test_execute_resets_description_before_new_query(self, cursor):
        cursor._query_result.description = [MagicMock()]
        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable()):
            cursor.execute("SELECT 1")
        assert cursor.description is None

    def test_executemany_server_side_binding_delegates_reset_to_execute(self, cursor):
        cursor._connection.paramstyle = ParamStyle.QMARK
        cursor._query_result.sqlstate = "42601"
        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable()):
            cursor.executemany("INSERT INTO t VALUES (?)", [(1,), (2,), (3,)])
        assert cursor.sqlstate is None

    def test_executemany_empty_params_does_not_reset(self, cursor):
        cursor._query_result = _QueryResult(rowcount=42)
        cursor.executemany("INSERT INTO t VALUES (?)", [])
        assert cursor._query_result.rowcount == 42


# ---------------------------------------------------------------------------
# Describe
# ---------------------------------------------------------------------------


class TestDescribe:
    """Unit tests for Cursor.describe method."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.config.numpy = False
        conn.paramstyle = ParamStyle.PYFORMAT
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_describe_returns_column_metadata(self, cursor):
        mock_desc = [MagicMock()]
        mock_desc[0].name = "COL1"
        qr = _QueryResult(description=mock_desc, sfqid="01abc-def", query="SELECT 1")

        with patch.object(BlockingImmutableCursor, "describe", return_value=qr):
            result = cursor.describe("SELECT 1 AS COL1")
        assert result is not None
        assert len(result) == 1
        assert result[0].name == "COL1"
        assert cursor.description == result

    def test_describe_returns_none_for_no_columns(self, cursor):
        qr = _QueryResult()
        with patch.object(BlockingImmutableCursor, "describe", return_value=qr):
            assert cursor.describe("INSERT INTO t VALUES (1)") is None

    def test_describe_side_effects_with_columns(self, cursor):
        mock_desc = [MagicMock()]
        mock_desc[0].name = "COL1"
        qr = _QueryResult(description=mock_desc, sfqid="01abc-def", query="SELECT 1", rowcount=0)

        cursor._query_result.rowcount = 42
        with patch.object(BlockingImmutableCursor, "describe", return_value=qr):
            cursor.describe("SELECT 1")
        assert cursor.sfqid == "01abc-def"
        assert cursor.query == "SELECT 1"
        assert cursor.rowcount == 0

    def test_describe_side_effects_without_columns(self, cursor):
        cursor._query_result.rowcount = 42
        qr = _QueryResult()
        with patch.object(BlockingImmutableCursor, "describe", return_value=qr):
            cursor.describe("SELECT 1")
        assert cursor.sfqid is None
        assert cursor.rowcount is None

    def test_describe_raises_when_closed(self, cursor, mock_connection):
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.describe("SELECT 1")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.describe("SELECT 1")

    def test_describe_propagates_prepare_error(self, cursor):
        with patch.object(
            BlockingImmutableCursor, "describe", side_effect=ProgrammingError("syntax error", sqlstate="42601")
        ):
            with pytest.raises(ProgrammingError):
                cursor.describe("INVALID SQL")
        assert cursor.sqlstate == "42601"


# ---------------------------------------------------------------------------
# query_result (from sfqid)
# ---------------------------------------------------------------------------


class TestQueryResult:
    """Unit tests for Cursor.query_result method."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.config.numpy = False
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_query_result_populates_cursor_state(self, cursor):
        mock_desc = [MagicMock()]
        mock_desc[0].name = "ID"
        immutable = _make_mock_immutable(
            description=mock_desc,
            rowcount=42,
            sqlstate="02000",
        )
        with patch.object(BlockingImmutableCursor, "from_async_query", return_value=immutable):
            ret = cursor.query_result("01234567-abcd-ef01-0000-000000000001")

        assert ret is cursor
        assert cursor.description is not None
        assert len(cursor.description) == 1
        assert cursor.description[0].name == "ID"
        assert cursor.rowcount == 42
        assert cursor.sqlstate == "02000"

    def test_query_result_resets_prior_state(self, cursor):
        cursor._query_result = _QueryResult(rowcount=99)
        cursor._binding_data = b"old"

        immutable = _make_mock_immutable()
        with patch.object(BlockingImmutableCursor, "from_async_query", return_value=immutable):
            cursor.query_result("qid")
        assert cursor._binding_data is None

    def test_query_result_raises_on_closed_cursor_or_connection(self, cursor, mock_connection):
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.query_result("qid")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.query_result("qid")

    def test_query_result_propagates_rpc_error(self, cursor):
        with patch.object(
            BlockingImmutableCursor, "from_async_query", side_effect=ProgrammingError("Query has expired")
        ):
            with pytest.raises(ProgrammingError, match="Query has expired"):
                cursor.query_result("expired-qid")


# ---------------------------------------------------------------------------
# QueryResultWaiter
# ---------------------------------------------------------------------------


class TestQueryResultWaiter:
    """Unit tests for QueryResultWaiter."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.is_still_running = MagicMock(side_effect=lambda s: s in (QueryStatus.RUNNING, QueryStatus.NO_DATA))
        return conn

    def test_returns_immediately_when_query_already_done(self, mock_connection):
        mock_connection.get_query_status_throw_if_error.return_value = QueryStatus.SUCCESS
        waiter = QueryResultWaiter(mock_connection, "qid")
        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep") as mock_sleep:
            waiter.wait()
        mock_sleep.assert_not_called()

    def test_polls_until_success(self, mock_connection):
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
        mock_connection.get_query_status_throw_if_error.side_effect = ProgrammingError("Query failed")
        waiter = QueryResultWaiter(mock_connection, "qid")
        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep"):
            with pytest.raises(ProgrammingError, match="Query failed"):
                waiter.wait()

    def test_raises_after_no_data_max_retry(self, mock_connection):
        mock_connection.get_query_status_throw_if_error.return_value = QueryStatus.NO_DATA
        waiter = QueryResultWaiter(mock_connection, "qid")
        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep"):
            with pytest.raises(DatabaseError, match="Cannot retrieve data"):
                waiter.wait()


# ---------------------------------------------------------------------------
# get_results_from_sfqid
# ---------------------------------------------------------------------------


class TestGetResultsFromSfqid:
    """Unit tests for Cursor.get_results_from_sfqid."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.get_query_status_throw_if_error.return_value = QueryStatus.SUCCESS
        conn.is_still_running.return_value = False
        conn.config.numpy = False
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sets_sfqid_eagerly(self, cursor):
        cursor.get_results_from_sfqid("test-qid")
        assert cursor.sfqid == "test-qid"

    def test_installs_prefetch_hook(self, cursor):
        cursor.get_results_from_sfqid("test-qid")
        assert cursor._prefetch_hook is not None

    def test_prefetch_hook_fires_on_fetch(self, cursor, mock_connection):
        with patch("snowflake.connector.cursor._query_result_waiter.time.sleep"):
            cursor.get_results_from_sfqid("test-qid")

        assert cursor._prefetch_hook is not None
        with patch.object(cursor, "query_result") as mock_qr:
            cursor._prefetch_hook()
        mock_qr.assert_called_once_with("test-qid")
        assert cursor._prefetch_hook is None

    def test_raises_on_closed_cursor(self, cursor):
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.get_results_from_sfqid("qid")

    def test_raises_when_query_already_failed(self, cursor, mock_connection):
        mock_connection.get_query_status_throw_if_error.side_effect = ProgrammingError("Query failed")
        with pytest.raises(ProgrammingError, match="Query failed"):
            cursor.get_results_from_sfqid("bad-qid")

    def test_execute_clears_pending_hook(self, cursor):
        cursor.get_results_from_sfqid("test-qid")
        assert cursor._prefetch_hook is not None

        with patch.object(BlockingImmutableCursor, "execute", return_value=_make_mock_immutable()):
            cursor.execute("SELECT 1")
        assert cursor._prefetch_hook is None


# ---------------------------------------------------------------------------
# abort_query
# ---------------------------------------------------------------------------


class TestAbortQuery:
    """Unit tests for Cursor.abort_query method."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_abort_query_returns_true_on_success(self, cursor):
        with patch.object(BlockingImmutableCursor, "abort_query", return_value=True):
            assert cursor.abort_query("01234567-abcd-ef01-0000-000000000001") is True

    def test_abort_query_returns_false_on_failure(self, cursor):
        with patch.object(BlockingImmutableCursor, "abort_query", return_value=False):
            assert cursor.abort_query("some-qid") is False

    def test_abort_query_does_not_mutate_cursor_state(self, cursor):
        with patch.object(BlockingImmutableCursor, "abort_query", return_value=True):
            cursor.abort_query("some-qid")
        assert cursor.description is None
        assert cursor.rowcount is None

    def test_abort_query_raises_on_closed_cursor_or_connection(self, cursor, mock_connection):
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.abort_query("qid")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.abort_query("qid")

    def test_abort_query_propagates_rpc_error(self, cursor):
        with patch.object(BlockingImmutableCursor, "abort_query", side_effect=ProgrammingError("Request failed")):
            with pytest.raises(ProgrammingError, match="Request failed"):
                cursor.abort_query("bad-qid")


# ---------------------------------------------------------------------------
# execute_async
# ---------------------------------------------------------------------------


class TestExecuteAsync:
    """Unit tests for Cursor.execute_async method."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = MagicMock()
        conn.is_closed.return_value = False
        conn.paramstyle = ParamStyle.PYFORMAT
        conn.config.numpy = False
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_returns_dict_with_query_id(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute_async", return_value="01abc-fake-query-id"):
            result = cursor.execute_async("SELECT 1")
        assert isinstance(result, dict)
        assert result["queryId"] == "01abc-fake-query-id"

    def test_sets_sfqid_on_cursor(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute_async", return_value="01abc-fake-query-id"):
            cursor.execute_async("SELECT 1")
        assert cursor.sfqid == "01abc-fake-query-id"

    def test_resets_cursor_state(self, cursor):
        cursor._binding_data = b"old"
        with patch.object(BlockingImmutableCursor, "execute_async", return_value="qid"):
            cursor.execute_async("SELECT 1")
        assert cursor._binding_data is None

    def test_raises_on_closed_cursor(self, cursor):
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.execute_async("SELECT 1")

    def test_propagates_rpc_error(self, cursor):
        with patch.object(
            BlockingImmutableCursor, "execute_async", side_effect=ProgrammingError("Async submission failed")
        ):
            with pytest.raises(ProgrammingError, match="Async submission failed"):
                cursor.execute_async("SELECT 1")

    def test_handles_empty_query_id(self, cursor):
        with patch.object(BlockingImmutableCursor, "execute_async", return_value=None):
            result = cursor.execute_async("SELECT 1")
        assert result["queryId"] is None


# ---------------------------------------------------------------------------
# Misc
# ---------------------------------------------------------------------------


class TestCursorFormatQueryForLog:
    """Unit tests for cursor._format_query_for_log delegation to connection."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        mock_connection._format_query_for_log.return_value = "formatted"
        return mock_connection

    def test_delegates_to_connection(self, mock_connection):
        cursor = SnowflakeCursor(mock_connection)
        result = cursor._format_query_for_log("SELECT * FROM big_table")
        mock_connection._format_query_for_log.assert_called_once_with("SELECT * FROM big_table")
        assert result == "formatted"
