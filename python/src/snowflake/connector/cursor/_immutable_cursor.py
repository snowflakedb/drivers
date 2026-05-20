"""Async-native, single-execution, consume-once cursor primitive.

:class:`ImmutableCursor` is the **shared** building block used by both the
sync user-facing cursor (``snowflake.connector.cursor.SnowflakeCursorBase``,
via :class:`BlockingImmutableCursor`) and the future async-first cursor
(``snowflake.connector.aio.cursor.SnowflakeCursor``). It lives under
``_internal`` because it is not part of the public API — users construct
the wrapping ``SnowflakeCursor``, never an ``ImmutableCursor`` directly.

Lifecycle (linear, one-way):

* ``CREATED``   — constructed, no query yet
* ``EXECUTED``  — query sent, ``ResultSetHandle`` owned, no rows consumed
* ``CONSUMING`` — row iterator started; further fetches return more rows
* ``EXHAUSTED`` — rows fully drained or cursor closed

There is no reset; each ``execute()`` call constructs a fresh instance.
Illegal transitions raise :class:`InterfaceError`.

Design note: the state machine is implemented imperatively rather than via
decorator annotations because most fetch methods transition **conditionally**
— ``fetchone`` may stay in ``CONSUMING`` or jump to ``EXHAUSTED`` depending
on whether the underlying iterator is drained. A static
``@transition(from_=X, to=Y)`` decorator cannot express "may transition,
depending on runtime data" without escape hatches that defeat the point.
"""

from __future__ import annotations

import logging

from collections.abc import Iterator
from enum import IntEnum
from typing import TYPE_CHECKING

from snowflake.connector._internal.api_client.client_api import (
    AsyncCoreDriver,
    async_core_driver,
)
from snowflake.connector._internal.arrow_stream_utils import create_row_iterator
from snowflake.connector._internal.config_utils import create_config_setting
from snowflake.connector._internal.errorcode import ER_NO_DATA_FOUND
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConfigSetting,
    ConnectionHandle,
    QueryBindings,
    ResultSetGetChunksResponse,
    ResultSetHandle,
    ResultSetResponse,
)
from snowflake.connector._internal.statement_utils import async_statement, get_stream_ptr
from snowflake.connector.cursor._query_result import _QueryResult
from snowflake.connector.cursor._result_metadata import QueryResultStats, ResultMetadata
from snowflake.connector.errors import InterfaceError, ProgrammingError


if TYPE_CHECKING:
    # Connection imports cursor at runtime (it constructs cursors), so a
    # top-level import here would be circular. TYPE_CHECKING is required.
    from snowflake.connector.connection import Connection


logger = logging.getLogger(__name__)

# Public PEP 249 row types — match aliases on SnowflakeCursorBase.
Row = tuple[object, ...]
DictRow = dict[str, object]
# Values acceptable to :func:`create_config_setting`.
StatementOptionValue = bool | int | str | float | bytes | None


class _State(IntEnum):
    """Linear lifecycle states for :class:`ImmutableCursor`."""

    CREATED = 0
    EXECUTED = 1
    CONSUMING = 2
    EXHAUSTED = 3


# Allowed forward transitions, keyed by source state. Any other (source, target)
# pair is treated as a programmer error and raises :class:`InterfaceError`.
_ALLOWED_TRANSITIONS: dict[_State, frozenset[_State]] = {
    _State.CREATED: frozenset({_State.EXECUTED}),
    _State.EXECUTED: frozenset({_State.CONSUMING, _State.EXHAUSTED}),
    _State.CONSUMING: frozenset({_State.CONSUMING, _State.EXHAUSTED}),
    _State.EXHAUSTED: frozenset(),
}


def _require_open_conn_handle(connection: Connection) -> ConnectionHandle:
    """Return the connection's handle or raise if it has been released.

    ``Connection.conn_handle`` is typed ``ConnectionHandle | None`` because
    it is cleared on close. Cursor operations are meaningless on a closed
    connection — surface that as an :class:`InterfaceError` instead of
    propagating a confusing protobuf type error from a downstream call.
    """
    handle: ConnectionHandle | None = connection.conn_handle
    if handle is None:
        raise InterfaceError(msg="Connection is closed")
    return handle


class ImmutableCursor:
    """Async, single-execution, consume-once cursor.

    Most callers should construct via :meth:`execute`, :meth:`from_query_id`,
    or :meth:`from_sfqid` rather than ``__init__`` directly. Each instance
    binds to one query; to run another, build another ``ImmutableCursor``.
    """

    __slots__ = (
        "_async_client",
        "_connection",
        "_iterator",
        "_multi_query_ids",
        "_query_result",
        "_result_set_handle",
        "_rownumber",
        "_state",
        "_use_dict_result",
        "_use_numpy",
    )

    def __init__(
        self,
        connection: Connection,
        *,
        async_client: AsyncCoreDriver | None = None,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> None:
        self._connection = connection
        self._async_client = async_client if async_client is not None else async_core_driver
        self._state: _State = _State.CREATED
        self._query_result: _QueryResult = _QueryResult()
        self._result_set_handle: ResultSetHandle | None = None
        self._rownumber: int = -1
        self._iterator: Iterator[Row | DictRow] | None = None
        self._use_dict_result = use_dict_result
        self._use_numpy = use_numpy
        # Populated during ``execute`` / ``from_async_query`` if the server
        # returned a multi-statement response. The wrapping cursor uses this
        # to drive ``nextset()`` — one ImmutableCursor instance per child.
        self._multi_query_ids: list[str] | None = None

    # -- state machine ----------------------------------------------------

    @property
    def state(self) -> _State:
        return self._state

    def _transition(self, target: _State) -> None:
        """Move the state machine to *target*, or raise on illegal transitions."""
        if target not in _ALLOWED_TRANSITIONS[self._state]:
            raise InterfaceError(msg=f"ImmutableCursor: illegal state transition {self._state.name} -> {target.name}")
        self._state = target

    def _require_state(self, *allowed: _State) -> None:
        if self._state not in allowed:
            allowed_names = ", ".join(s.name for s in allowed)
            raise InterfaceError(
                msg=(
                    f"ImmutableCursor: operation requires state in [{allowed_names}], "
                    f"current state is {self._state.name}"
                )
            )

    # -- read-only properties (no FFI, safe in any state) -----------------

    @property
    def description(self) -> list[ResultMetadata] | None:
        return self._query_result.description

    @property
    def rowcount(self) -> int | None:
        return self._query_result.rowcount

    @property
    def sfqid(self) -> str | None:
        return self._query_result.sfqid

    @property
    def query(self) -> str | None:
        return self._query_result.query

    @property
    def sqlstate(self) -> str | None:
        return self._query_result.sqlstate

    @property
    def stats(self) -> QueryResultStats:
        return self._query_result.stats

    @property
    def rownumber(self) -> int:
        return self._rownumber

    @property
    def query_result(self) -> _QueryResult:
        """The metadata bag for this cursor's query."""
        return self._query_result

    @property
    def multi_query_ids(self) -> list[str] | None:
        """List of child query IDs when ``execute`` ran a multi-statement.

        ``None`` for single-statement queries. The wrapping sync/async cursor
        uses this to drive ``nextset()`` — one ``ImmutableCursor`` per child.
        """
        return self._multi_query_ids

    # -- async creation factories ----------------------------------------

    @classmethod
    async def execute(
        cls,
        connection: Connection,
        query: str,
        bindings: QueryBindings | None = None,
        *,
        statement_options: dict[str, StatementOptionValue] | None = None,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> ImmutableCursor:
        """Run *query* against *connection* and return an EXECUTED cursor.

        *query* must be the final, paramstyle-resolved SQL text and *bindings*
        a pre-built ``QueryBindings`` (or ``None``). Query preparation —
        client-side interpolation vs server-side bindings — is the wrapping
        cursor's responsibility, not this primitive's; that logic depends on
        :attr:`Connection.paramstyle` and other state ImmutableCursor does
        not own.

        Multi-statement queries: this primitive only adopts the **first**
        child result set. Navigation across remaining children is the
        wrapping cursor's job — it should call :meth:`from_query_id` for
        each subsequent child query ID.
        """
        cursor = cls(connection, use_dict_result=use_dict_result, use_numpy=use_numpy)
        conn_handle = _require_open_conn_handle(connection)

        async with async_statement(conn_handle, query) as stmt_handle:
            if statement_options:
                opts: dict[str, ConfigSetting] = {}
                for key, value in statement_options.items():
                    setting = create_config_setting(value)
                    if setting is not None:
                        opts[key] = setting
                if opts:
                    await cursor._async_client.statement_set_options(stmt_handle=stmt_handle, options=opts)

            response = await cursor._async_client.statement_execute_query(stmt_handle=stmt_handle, bindings=bindings)
            if response.HasField("multi"):
                cursor._multi_query_ids = list(response.multi.query_ids)
                first_qid = response.multi.query_ids[0] if response.multi.query_ids else None
                if first_qid is None:
                    cursor._query_result = _QueryResult(query=query)
                else:
                    rs = await cursor._async_client.connection_get_result_set(
                        conn_handle=conn_handle, query_id=first_qid
                    )
                    cursor._adopt_result_set(rs, query)
            else:
                cursor._adopt_result_set(response.single, query)

        cursor._transition(_State.EXECUTED)
        return cursor

    @classmethod
    async def from_query_id(
        cls,
        connection: Connection,
        query_id: str,
        *,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> ImmutableCursor:
        """Construct an EXECUTED cursor from an already-completed query ID.

        Used for fetching results of an async-submitted query, advancing
        through multi-statement results, or replaying a finished query
        identified by its server-side ID.
        """
        cursor = cls(connection, use_dict_result=use_dict_result, use_numpy=use_numpy)
        conn_handle = _require_open_conn_handle(connection)
        rs = await cursor._async_client.connection_get_result_set(conn_handle=conn_handle, query_id=query_id)
        cursor._adopt_result_set(rs, query=None)
        cursor._transition(_State.EXECUTED)
        return cursor

    @classmethod
    async def from_async_query(
        cls,
        connection: Connection,
        query_id: str,
        *,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> ImmutableCursor:
        """Construct an EXECUTED cursor from a previously async-submitted query.

        Differs from :meth:`from_query_id`: this calls
        ``connection_get_query_result`` (which handles single/multi response
        shape just like a freshly executed query), whereas ``from_query_id``
        calls ``connection_get_result_set`` (assumes a single result set
        already exists). Use this for the ``execute_async`` -> poll ->
        retrieve workflow.
        """
        cursor = cls(connection, use_dict_result=use_dict_result, use_numpy=use_numpy)
        conn_handle = _require_open_conn_handle(connection)
        response = await cursor._async_client.connection_get_query_result(conn_handle=conn_handle, query_id=query_id)
        if response.HasField("multi"):
            cursor._multi_query_ids = list(response.multi.query_ids)
            first_qid = response.multi.query_ids[0] if response.multi.query_ids else None
            if first_qid is None:
                cursor._query_result = _QueryResult()
            else:
                rs = await cursor._async_client.connection_get_result_set(conn_handle=conn_handle, query_id=first_qid)
                cursor._adopt_result_set(rs, query=None)
        else:
            cursor._adopt_result_set(response.single, query=None)
        cursor._transition(_State.EXECUTED)
        return cursor

    @staticmethod
    async def execute_async(
        connection: Connection,
        query: str,
        bindings: QueryBindings | None = None,
        *,
        statement_options: dict[str, StatementOptionValue] | None = None,
    ) -> str | None:
        """Submit *query* for async execution and return the server-side query ID.

        Does not construct an ImmutableCursor — the query is in flight on the
        server and has not produced a result set yet. Pair with
        :meth:`from_async_query` (after polling for completion) to retrieve
        results.
        """
        conn_handle = _require_open_conn_handle(connection)

        async with async_statement(conn_handle, query) as stmt_handle:
            if statement_options:
                opts: dict[str, ConfigSetting] = {}
                for key, value in statement_options.items():
                    setting = create_config_setting(value)
                    if setting is not None:
                        opts[key] = setting
                if opts:
                    await async_core_driver.statement_set_options(stmt_handle=stmt_handle, options=opts)

            response = await async_core_driver.statement_execute_async(stmt_handle=stmt_handle, bindings=bindings)
        return response.query_id if response.query_id else None

    @staticmethod
    async def abort_query(connection: Connection, query_id: str) -> bool:
        """Cancel an in-flight async query identified by *query_id*.

        Returns the server-side ``success`` flag.
        """
        conn_handle = _require_open_conn_handle(connection)
        response = await async_core_driver.connection_abort_query(conn_handle=conn_handle, query_id=query_id)
        return bool(response.success)

    @staticmethod
    async def describe(connection: Connection, query: str) -> _QueryResult:
        """Prepare *query* and return its result-set metadata only.

        Issues ``statement_prepare`` against the server, reads back column
        metadata, and returns a populated :class:`_QueryResult`. No
        ``ResultSetHandle`` is acquired — the query is not executed and no
        cursor is constructed.
        """
        conn_handle = _require_open_conn_handle(connection)
        async with async_statement(conn_handle, query) as stmt_handle:
            response = await async_core_driver.statement_prepare(stmt_handle=stmt_handle)
        return _QueryResult.from_prepare_result(response.result)

    def _adopt_result_set(self, response: ResultSetResponse, query: str | None) -> None:
        """Capture the ResultSetHandle and metadata from a ResultSetResponse."""
        self._result_set_handle = response.result_set_handle
        self._query_result = _QueryResult.from_result_set_response(response, query)

    # -- async fetch methods ---------------------------------------------

    async def _ensure_iterator(self) -> Iterator[Row | DictRow]:
        """Lazily build the row iterator on first fetch.

        Transitions ``EXECUTED -> CONSUMING``. Subsequent fetches reuse the
        already-built iterator. Raises if called from any other state.
        """
        if self._state is _State.EXECUTED:
            await self._build_iterator()
            self._transition(_State.CONSUMING)
        else:
            self._require_state(_State.EXECUTED, _State.CONSUMING)
        # _build_iterator always populates _iterator on success; if we reach
        # here without one, treat it as a programmer error rather than an
        # assertion (asserts are stripped under -O and forbidden in production).
        iterator = self._iterator
        if iterator is None:
            raise InterfaceError(msg="ImmutableCursor: row iterator was not initialised before fetch")
        return iterator

    async def _build_iterator(self) -> None:
        """Materialise a row iterator from the held ``ResultSetHandle``.

        Performs one async FFI call to obtain the Arrow stream pointer, then
        wraps it with the existing sync ``create_row_iterator`` helper. The
        iterator iterates in-process — no further FFI per row.
        """
        if self._result_set_handle is None:
            raise ProgrammingError(
                msg="No results available (not produced by this query)",
                errno=ER_NO_DATA_FOUND,
            )
        response = await self._async_client.result_set_get_stream(result_set_handle=self._result_set_handle)
        stream_ptr = get_stream_ptr(response)
        self._iterator = create_row_iterator(
            stream_ptr,
            self._connection,
            use_dict_result=self._use_dict_result,
            use_numpy=self._use_numpy,
        )

    async def fetchone(self) -> Row | DictRow | None:
        iterator = await self._ensure_iterator()
        try:
            row = next(iterator)
        except StopIteration:
            self._transition(_State.EXHAUSTED)
            return None
        self._rownumber += 1
        return row

    async def fetchmany(self, size: int | None = None) -> list[Row | DictRow]:
        if size is None:
            size = 1
        iterator = await self._ensure_iterator()
        rows: list[Row | DictRow] = []
        for _ in range(size):
            try:
                rows.append(next(iterator))
                self._rownumber += 1
            except StopIteration:
                self._transition(_State.EXHAUSTED)
                break
        return rows

    async def fetchall(self) -> list[Row | DictRow]:
        iterator = await self._ensure_iterator()
        rows: list[Row | DictRow] = list(iterator)
        self._rownumber += len(rows)
        self._transition(_State.EXHAUSTED)
        return rows

    async def get_arrow_stream_ptr(self) -> int:
        """Return a raw C pointer to a fresh Arrow stream over the result set.

        Each call builds an independent stream from the stored ``RowsetData``,
        so it is safe to call multiple times (e.g. ``fetch_arrow_batches``
        after a partial ``fetchone`` sequence). The caller owns the stream.

        Requires ``EXECUTED`` or ``CONSUMING`` state — the result-set handle
        must still be alive.
        """
        self._require_state(_State.EXECUTED, _State.CONSUMING)
        if self._result_set_handle is None:
            raise ProgrammingError(
                msg="No results available (not produced by this query)",
                errno=ER_NO_DATA_FOUND,
            )
        response = await self._async_client.result_set_get_stream(result_set_handle=self._result_set_handle)
        return get_stream_ptr(response)

    async def get_chunks(self) -> ResultSetGetChunksResponse | None:
        """Return chunk metadata for distributed fetch, or ``None`` if unavailable.

        Safe in ``EXECUTED`` or ``CONSUMING`` state. Returns ``None`` when no
        result-set handle is held (e.g. after close or for DML queries).
        """
        self._require_state(_State.EXECUTED, _State.CONSUMING)
        if self._result_set_handle is None:
            return None
        return await self._async_client.result_set_get_chunks(result_set_handle=self._result_set_handle)

    async def close(self) -> None:
        """Release the held ``ResultSetHandle`` and mark the cursor exhausted.

        Idempotent. After this returns the cursor is in ``EXHAUSTED`` state
        and any further fetch will raise via :meth:`_require_state`.
        """
        if self._state is _State.EXHAUSTED:
            return
        handle = self._result_set_handle
        self._result_set_handle = None
        if handle is not None:
            try:
                await self._async_client.result_set_release(result_set_handle=handle)
            except Exception:
                logger.warning("Failed to release ResultSet handle", exc_info=True)
        # Force-transition to EXHAUSTED regardless of where we were.
        self._state = _State.EXHAUSTED
