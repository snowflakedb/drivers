"""Unit tests for async cursor classes.

Tests mock at the :class:`ImmutableCursor` boundary — the async cursor never
touches the FFI layer.
"""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from snowflake.connector.aio.cursor import AsyncDictCursor, AsyncSnowflakeCursor, AsyncSnowflakeCursorBase
from snowflake.connector.cursor._immutable_cursor import ImmutableCursor
from snowflake.connector.cursor._query_result import _QueryResult
from snowflake.connector.cursor._result_metadata import QueryResultStats
from snowflake.connector.errors import InterfaceError, ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


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
    """Build a mock :class:`ImmutableCursor` with canned async fetch behaviour."""
    mock = AsyncMock(spec=ImmutableCursor)

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

    async def _fetchone():
        if pos[0] >= len(rows):
            return None
        row = rows[pos[0]]
        pos[0] += 1
        rownumber[0] = pos[0] - 1
        mock.rownumber = rownumber[0]
        return row

    async def _fetchmany(size=None):
        if size is None:
            size = 1
        batch = rows[pos[0] : pos[0] + size]
        pos[0] += len(batch)
        if batch:
            rownumber[0] = pos[0] - 1
            mock.rownumber = rownumber[0]
        return batch

    async def _fetchall():
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
    cursor._immutable = mock_immutable
    cursor._query_result = mock_immutable.query_result


def _mock_connection():
    conn = MagicMock()
    conn.is_closed.return_value = False
    conn.config.numpy = False
    return conn


# ---------------------------------------------------------------------------
# Basic fetch tests
# ---------------------------------------------------------------------------


class TestAsyncFetchone:
    @pytest.fixture
    def cursor(self):
        return AsyncSnowflakeCursor(_mock_connection())

    @pytest.mark.asyncio
    async def test_fetchone_returns_single_row(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert await cursor.fetchone() == (1,)

    @pytest.mark.asyncio
    async def test_fetchone_returns_none_when_exhausted(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([]))
        assert await cursor.fetchone() is None

    @pytest.mark.asyncio
    async def test_fetchone_sequential_calls(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert await cursor.fetchone() == (1,)
        assert await cursor.fetchone() == (2,)
        assert await cursor.fetchone() == (3,)
        assert await cursor.fetchone() is None


class TestAsyncFetchall:
    @pytest.fixture
    def cursor(self):
        return AsyncSnowflakeCursor(_mock_connection())

    @pytest.mark.asyncio
    async def test_fetchall_returns_all_rows(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        assert await cursor.fetchall() == [(1,), (2,), (3,)]

    @pytest.mark.asyncio
    async def test_fetchall_returns_empty_list_when_no_rows(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([]))
        assert await cursor.fetchall() == []

    @pytest.mark.asyncio
    async def test_fetchall_after_partial_fetchone(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        await cursor.fetchone()
        await cursor.fetchone()
        assert await cursor.fetchall() == [(3,), (4,), (5,)]


class TestAsyncFetchmany:
    @pytest.fixture
    def cursor(self):
        return AsyncSnowflakeCursor(_mock_connection())

    @pytest.mark.asyncio
    async def test_fetchmany_default_uses_arraysize(self, cursor):
        cursor.arraysize = 3
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        assert await cursor.fetchmany() == [(1,), (2,), (3,)]

    @pytest.mark.asyncio
    async def test_fetchmany_with_explicit_size(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        assert await cursor.fetchmany(2) == [(1,), (2,)]

    @pytest.mark.asyncio
    async def test_fetchmany_negative_size_raises(self, cursor):
        with pytest.raises(ProgrammingError, match="not zero or positive"):
            await cursor.fetchmany(-1)

    @pytest.mark.asyncio
    async def test_fetchmany_sequential_calls(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        assert await cursor.fetchmany(2) == [(1,), (2,)]
        assert await cursor.fetchmany(2) == [(3,), (4,)]
        assert await cursor.fetchmany(2) == [(5,)]


# ---------------------------------------------------------------------------
# Dict cursor
# ---------------------------------------------------------------------------


class TestAsyncDictCursor:
    @pytest.fixture
    def cursor(self):
        return AsyncDictCursor(_mock_connection())

    @pytest.mark.asyncio
    async def test_fetchone_returns_dict(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([{"ID": 1, "NAME": "alice"}]))
        row = await cursor.fetchone()
        assert row == {"ID": 1, "NAME": "alice"}
        assert isinstance(row, dict)

    @pytest.mark.asyncio
    async def test_fetchall_returns_list_of_dicts(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([{"ID": 1}, {"ID": 2}]))
        assert await cursor.fetchall() == [{"ID": 1}, {"ID": 2}]


# ---------------------------------------------------------------------------
# Rownumber
# ---------------------------------------------------------------------------


class TestAsyncRownumber:
    @pytest.fixture
    def cursor(self):
        return AsyncSnowflakeCursor(_mock_connection())

    @pytest.mark.asyncio
    async def test_rownumber_increments_with_fetchone(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        await cursor.fetchone()
        assert cursor.rownumber == 0
        await cursor.fetchone()
        assert cursor.rownumber == 1
        await cursor.fetchone()
        assert cursor.rownumber == 2

    @pytest.mark.asyncio
    async def test_rownumber_updated_by_fetchall(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,), (4,), (5,)]))
        await cursor.fetchall()
        assert cursor.rownumber == 4


# ---------------------------------------------------------------------------
# Async iteration (async for)
# ---------------------------------------------------------------------------


class TestAsyncIteration:
    @pytest.fixture
    def cursor(self):
        return AsyncSnowflakeCursor(_mock_connection())

    @pytest.mark.asyncio
    async def test_async_for(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([(1,), (2,), (3,)]))
        collected = []
        async for row in cursor:
            collected.append(row)
        assert collected == [(1,), (2,), (3,)]

    @pytest.mark.asyncio
    async def test_async_for_empty_result(self, cursor):
        _inject_immutable(cursor, _make_mock_immutable([]))
        collected = []
        async for row in cursor:
            collected.append(row)
        assert collected == []


# ---------------------------------------------------------------------------
# Async context manager
# ---------------------------------------------------------------------------


class TestAsyncContextManager:
    @pytest.mark.asyncio
    async def test_async_with_closes_cursor(self):
        conn = _mock_connection()
        async with AsyncSnowflakeCursor(conn) as cur:
            assert not cur._closed
        assert cur._closed is True

    @pytest.mark.asyncio
    async def test_async_with_returns_cursor(self):
        conn = _mock_connection()
        async with AsyncSnowflakeCursor(conn) as cur:
            assert isinstance(cur, AsyncSnowflakeCursorBase)


# ---------------------------------------------------------------------------
# Execute lifecycle
# ---------------------------------------------------------------------------


class TestAsyncExecuteLifecycle:
    @pytest.fixture
    def cursor(self):
        conn = _mock_connection()
        conn.conn_handle = MagicMock()
        conn.paramstyle = MagicMock()
        conn.paramstyle.is_client_side.return_value = True
        return AsyncSnowflakeCursor(conn)

    @pytest.mark.asyncio
    async def test_execute_creates_immutable(self, cursor):
        immutable = _make_mock_immutable(sfqid="test-qid")
        with patch.object(ImmutableCursor, "execute", return_value=immutable):
            await cursor.execute("SELECT 1")
        assert cursor._immutable is immutable
        assert cursor.sfqid == "test-qid"

    @pytest.mark.asyncio
    async def test_execute_resets_previous_state(self, cursor):
        old_immutable = _make_mock_immutable()
        _inject_immutable(cursor, old_immutable)

        new_immutable = _make_mock_immutable()
        with patch.object(ImmutableCursor, "execute", return_value=new_immutable):
            await cursor.execute("SELECT 1")
        old_immutable.close.assert_awaited_once()

    @pytest.mark.asyncio
    async def test_execute_propagates_error_and_captures_metadata(self, cursor):
        with patch.object(ImmutableCursor, "execute", side_effect=ProgrammingError("bad sql", sqlstate="42601")):
            with pytest.raises(ProgrammingError):
                await cursor.execute("INVALID SQL")
        assert cursor.sqlstate == "42601"


# ---------------------------------------------------------------------------
# Close / reset
# ---------------------------------------------------------------------------


class TestAsyncClose:
    @pytest.mark.asyncio
    async def test_close_returns_true_on_success(self):
        cur = AsyncSnowflakeCursor(_mock_connection())
        assert await cur.close() is True

    @pytest.mark.asyncio
    async def test_close_returns_false_when_already_closed(self):
        cur = AsyncSnowflakeCursor(_mock_connection())
        await cur.close()
        assert await cur.close() is False

    @pytest.mark.asyncio
    async def test_close_closes_immutable(self):
        cur = AsyncSnowflakeCursor(_mock_connection())
        immutable = _make_mock_immutable()
        _inject_immutable(cur, immutable)
        await cur.close()
        immutable.close.assert_awaited_once()

    @pytest.mark.asyncio
    async def test_closed_cursor_raises_on_fetch(self):
        cur = AsyncSnowflakeCursor(_mock_connection())
        await cur.close()
        with pytest.raises(InterfaceError, match="Cursor is closed"):
            await cur.fetchone()


# ---------------------------------------------------------------------------
# Properties shared via mixin
# ---------------------------------------------------------------------------


class TestAsyncMixinProperties:
    @pytest.fixture
    def cursor(self):
        return AsyncSnowflakeCursor(_mock_connection())

    def test_description_before_execute(self, cursor):
        assert cursor.description is None

    def test_rowcount_before_execute(self, cursor):
        assert cursor.rowcount is None

    def test_arraysize_default(self, cursor):
        assert cursor.arraysize == 1

    def test_arraysize_settable(self, cursor):
        cursor.arraysize = 10
        assert cursor.arraysize == 10

    def test_lastrowid_is_none(self, cursor):
        assert cursor.lastrowid is None

    def test_rownumber_none_before_fetch(self, cursor):
        assert cursor.rownumber is None

    def test_sqlstate_none_before_execute(self, cursor):
        assert cursor.sqlstate is None

    def test_stats_default(self, cursor):
        assert cursor.stats == QueryResultStats()

    def test_is_closed_false_initially(self, cursor):
        assert not cursor.is_closed()

    @pytest.mark.asyncio
    async def test_is_closed_true_after_close(self, cursor):
        await cursor.close()
        assert cursor.is_closed()
