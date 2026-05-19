"""Synchronous wrapper around :class:`ImmutableCursor`.

Bridges every async ``ImmutableCursor`` method to a sync call using the
process-wide background event loop from
:func:`~snowflake.connector._internal.api_client.client_api.get_background_loop`.
Read-only properties proxy through with zero overhead — they are pure Python
attribute reads on the wrapped cursor.

Like :class:`ImmutableCursor`, this is **internal**. It is consumed by the
public sync ``SnowflakeCursorBase`` (and not by user code directly).
"""

from __future__ import annotations

import asyncio

from collections.abc import Coroutine
from typing import TYPE_CHECKING, TypeVar

from snowflake.connector._internal.api_client.client_api import get_background_loop
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    QueryBindings,
    ResultSetGetChunksResponse,
)
from snowflake.connector.cursor._immutable_cursor import (
    DictRow,
    ImmutableCursor,
    Row,
    StatementOptionValue,
    _State,
)
from snowflake.connector.cursor._query_result import _QueryResult
from snowflake.connector.cursor._result_metadata import QueryResultStats, ResultMetadata


if TYPE_CHECKING:
    # Connection imports cursor at runtime, so a top-level import here would
    # be circular. Only this one stays under TYPE_CHECKING.
    from snowflake.connector.connection import Connection


_T = TypeVar("_T")


class BlockingImmutableCursor:
    """Sync facade over an :class:`ImmutableCursor`.

    Each blocking method submits the corresponding coroutine to the shared
    background event loop via ``asyncio.run_coroutine_threadsafe(...).result()``.
    Properties proxy directly to the wrapped cursor (no event-loop hop, no FFI).
    """

    __slots__ = ("_async",)

    def __init__(self, async_cursor: ImmutableCursor) -> None:
        self._async = async_cursor

    # -- async-to-sync bridge --------------------------------------------

    @staticmethod
    def _run(coro: Coroutine[object, object, _T]) -> _T:
        return asyncio.run_coroutine_threadsafe(coro, get_background_loop()).result()

    # -- sync creation factories -----------------------------------------

    @classmethod
    def execute(
        cls,
        connection: Connection,
        query: str,
        bindings: QueryBindings | None = None,
        *,
        statement_options: dict[str, StatementOptionValue] | None = None,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> BlockingImmutableCursor:
        return cls(
            cls._run(
                ImmutableCursor.execute(
                    connection,
                    query,
                    bindings,
                    statement_options=statement_options,
                    use_dict_result=use_dict_result,
                    use_numpy=use_numpy,
                )
            )
        )

    @classmethod
    def from_query_id(
        cls,
        connection: Connection,
        query_id: str,
        *,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> BlockingImmutableCursor:
        return cls(
            cls._run(
                ImmutableCursor.from_query_id(
                    connection,
                    query_id,
                    use_dict_result=use_dict_result,
                    use_numpy=use_numpy,
                )
            )
        )

    @classmethod
    def from_async_query(
        cls,
        connection: Connection,
        query_id: str,
        *,
        use_dict_result: bool = False,
        use_numpy: bool = False,
    ) -> BlockingImmutableCursor:
        return cls(
            cls._run(
                ImmutableCursor.from_async_query(
                    connection,
                    query_id,
                    use_dict_result=use_dict_result,
                    use_numpy=use_numpy,
                )
            )
        )

    @staticmethod
    def execute_async(
        connection: Connection,
        query: str,
        bindings: QueryBindings | None = None,
        *,
        statement_options: dict[str, StatementOptionValue] | None = None,
    ) -> str | None:
        """Submit *query* asynchronously; returns the server-side query ID.

        Static — no cursor is constructed.
        """
        return BlockingImmutableCursor._run(
            ImmutableCursor.execute_async(connection, query, bindings, statement_options=statement_options)
        )

    @staticmethod
    def abort_query(connection: Connection, query_id: str) -> bool:
        return BlockingImmutableCursor._run(ImmutableCursor.abort_query(connection, query_id))

    @staticmethod
    def describe(connection: Connection, query: str) -> _QueryResult:
        """Prepare *query* and return its metadata-only :class:`_QueryResult`."""
        return BlockingImmutableCursor._run(ImmutableCursor.describe(connection, query))

    # -- sync fetch methods ----------------------------------------------

    def fetchone(self) -> Row | DictRow | None:
        return self._run(self._async.fetchone())

    def fetchmany(self, size: int | None = None) -> list[Row | DictRow]:
        return self._run(self._async.fetchmany(size))

    def fetchall(self) -> list[Row | DictRow]:
        return self._run(self._async.fetchall())

    def get_arrow_stream_ptr(self) -> int:
        return self._run(self._async.get_arrow_stream_ptr())

    def get_chunks(self) -> ResultSetGetChunksResponse | None:
        return self._run(self._async.get_chunks())

    def close(self) -> None:
        self._run(self._async.close())

    # -- read-only properties (direct proxies, no async hop) -------------

    @property
    def description(self) -> list[ResultMetadata] | None:
        return self._async.description

    @property
    def rowcount(self) -> int | None:
        return self._async.rowcount

    @property
    def sfqid(self) -> str | None:
        return self._async.sfqid

    @property
    def query(self) -> str | None:
        return self._async.query

    @property
    def sqlstate(self) -> str | None:
        return self._async.sqlstate

    @property
    def stats(self) -> QueryResultStats:
        return self._async.stats

    @property
    def rownumber(self) -> int:
        return self._async.rownumber

    @property
    def query_result(self) -> _QueryResult:
        return self._async.query_result

    @property
    def multi_query_ids(self) -> list[str] | None:
        return self._async.multi_query_ids

    @property
    def state(self) -> _State:
        return self._async.state
