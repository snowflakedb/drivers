"""Unit tests for :class:`ImmutableCursor` and its sync wrapper."""

from __future__ import annotations

import asyncio

from unittest.mock import AsyncMock, MagicMock

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    ResultSetHandle,
)
from snowflake.connector.cursor._blocking_immutable_cursor import (
    BlockingImmutableCursor,
)
from snowflake.connector.cursor._immutable_cursor import (
    DictRow,
    ImmutableCursor,
    Row,
    _State,
)
from snowflake.connector.errors import InterfaceError
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


def _make_async_client_mock() -> AsyncMock:
    return AsyncMock()


def _make_connection_mock() -> MagicMock:
    conn = MagicMock()
    conn.conn_handle = ConnectionHandle(id=42)
    return conn


class TestStateMachine:
    def test_initial_state_is_created(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        assert cursor.state is _State.CREATED

    def test_transition_created_to_executed_allowed(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._transition(_State.EXECUTED)
        assert cursor.state is _State.EXECUTED

    def test_transition_executed_to_consuming_allowed(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._transition(_State.EXECUTED)
        cursor._transition(_State.CONSUMING)
        assert cursor.state is _State.CONSUMING

    def test_transition_consuming_to_exhausted_allowed(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._transition(_State.EXECUTED)
        cursor._transition(_State.CONSUMING)
        cursor._transition(_State.EXHAUSTED)
        assert cursor.state is _State.EXHAUSTED

    def test_transition_executed_to_exhausted_allowed(self) -> None:
        """Skipping CONSUMING (e.g. close before any fetch) is legal."""
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._transition(_State.EXECUTED)
        cursor._transition(_State.EXHAUSTED)
        assert cursor.state is _State.EXHAUSTED

    def test_transition_consuming_to_consuming_allowed(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._transition(_State.EXECUTED)
        cursor._transition(_State.CONSUMING)
        cursor._transition(_State.CONSUMING)
        assert cursor.state is _State.CONSUMING

    @pytest.mark.parametrize(
        "from_state,to_state",
        [
            (_State.CREATED, _State.CONSUMING),
            (_State.CREATED, _State.EXHAUSTED),
            (_State.CREATED, _State.CREATED),
            (_State.EXECUTED, _State.CREATED),
            (_State.EXECUTED, _State.EXECUTED),
            (_State.CONSUMING, _State.CREATED),
            (_State.CONSUMING, _State.EXECUTED),
            (_State.EXHAUSTED, _State.CREATED),
            (_State.EXHAUSTED, _State.EXECUTED),
            (_State.EXHAUSTED, _State.CONSUMING),
            (_State.EXHAUSTED, _State.EXHAUSTED),
        ],
    )
    def test_illegal_transitions_raise(self, from_state: _State, to_state: _State) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._state = from_state
        with pytest.raises(InterfaceError, match="illegal state transition"):
            cursor._transition(to_state)
        assert cursor.state is from_state, "state must not change when transition is rejected"

    def test_require_state_raises_when_mismatch(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._state = _State.CREATED
        with pytest.raises(InterfaceError, match="operation requires state"):
            cursor._require_state(_State.EXECUTED)

    def test_require_state_passes_when_match(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._state = _State.EXECUTED
        cursor._require_state(_State.EXECUTED, _State.CONSUMING)


class TestFetchTransitions:
    """Verify fetch methods transition state correctly.

    Pre-set ``_state=CONSUMING`` and pre-populate ``_iterator`` to bypass
    ``_build_iterator`` (which needs a real ResultSetHandle and FFI).
    State-machine behaviour is what's under test here.
    """

    @staticmethod
    def _cursor_with_rows(rows: list[Row | DictRow]) -> ImmutableCursor:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._state = _State.CONSUMING
        cursor._iterator = iter(rows)
        return cursor

    def test_fetchone_returns_row_in_consuming(self) -> None:
        async def run() -> None:
            cursor = self._cursor_with_rows([("row",)])
            row = await cursor.fetchone()
            assert row == ("row",)
            assert cursor.state is _State.CONSUMING
            assert cursor.rownumber == 0

        asyncio.run(run())

    def test_fetchone_returns_none_at_end_and_transitions_to_exhausted(self) -> None:
        async def run() -> None:
            cursor = self._cursor_with_rows([])
            row = await cursor.fetchone()
            assert row is None
            assert cursor.state is _State.EXHAUSTED

        asyncio.run(run())

    def test_fetchall_drains_and_transitions_to_exhausted(self) -> None:
        async def run() -> None:
            cursor = self._cursor_with_rows([("a",), ("b",), ("c",)])
            rows = await cursor.fetchall()
            assert rows == [("a",), ("b",), ("c",)]
            assert cursor.state is _State.EXHAUSTED
            # rownumber is the index of the last fetched row (0-based).
            assert cursor.rownumber == 2

        asyncio.run(run())

    def test_fetchmany_partial_does_not_exhaust(self) -> None:
        async def run() -> None:
            cursor = self._cursor_with_rows([("a",), ("b",), ("c",)])
            rows = await cursor.fetchmany(2)
            assert rows == [("a",), ("b",)]
            assert cursor.state is _State.CONSUMING
            assert cursor.rownumber == 1

        asyncio.run(run())

    def test_fetchmany_runs_off_end_transitions_to_exhausted(self) -> None:
        async def run() -> None:
            cursor = self._cursor_with_rows([("a",)])
            rows = await cursor.fetchmany(5)
            assert rows == [("a",)]
            assert cursor.state is _State.EXHAUSTED

        asyncio.run(run())

    def test_fetchall_after_exhausted_raises(self) -> None:
        async def run() -> None:
            cursor = self._cursor_with_rows([])
            cursor._state = _State.EXHAUSTED
            with pytest.raises(InterfaceError, match="operation requires state"):
                await cursor.fetchall()

        asyncio.run(run())


class TestClose:
    def test_close_idempotent(self) -> None:
        async def run() -> None:
            client = _make_async_client_mock()
            cursor = ImmutableCursor(_make_connection_mock(), async_client=client)
            cursor._state = _State.EXECUTED
            await cursor.close()
            assert cursor.state is _State.EXHAUSTED
            await cursor.close()
            assert cursor.state is _State.EXHAUSTED

        asyncio.run(run())

    def test_close_releases_handle_when_present(self) -> None:
        async def run() -> None:
            client = _make_async_client_mock()
            cursor = ImmutableCursor(_make_connection_mock(), async_client=client)
            cursor._state = _State.EXECUTED
            cursor._result_set_handle = ResultSetHandle(id=99)

            await cursor.close()
            client.result_set_release.assert_awaited_once()
            assert cursor._result_set_handle is None

        asyncio.run(run())


class TestProperties:
    """Sync read-only properties must be safe in any state and not invoke FFI."""

    def test_properties_safe_in_created_state(self) -> None:
        client = _make_async_client_mock()
        cursor = ImmutableCursor(_make_connection_mock(), async_client=client)
        assert cursor.description is None
        assert cursor.rowcount is None
        assert cursor.sfqid is None
        assert cursor.query is None
        assert cursor.sqlstate is None
        assert cursor.rownumber == -1
        # No FFI calls should have happened.
        client.result_set_get_stream.assert_not_called()
        client.statement_execute_query.assert_not_called()

    def test_blocking_wrapper_proxies_properties(self) -> None:
        cursor = ImmutableCursor(_make_connection_mock(), async_client=_make_async_client_mock())
        cursor._query_result.sfqid = "test-qid-123"
        cursor._query_result.query = "SELECT 1"
        cursor._query_result.rowcount = 7
        wrapper = BlockingImmutableCursor(cursor)
        assert wrapper.sfqid == "test-qid-123"
        assert wrapper.query == "SELECT 1"
        assert wrapper.rowcount == 7
        # The wrapper's `state` proxies to the inner cursor.
        assert wrapper.state is _State.CREATED
