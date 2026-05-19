"""Async cursor classes.

``AsyncSnowflakeCursorBase`` mirrors :class:`SnowflakeCursorBase` but uses
``async def`` for every method that crosses the FFI boundary, operating
directly on :class:`ImmutableCursor` (the async primitive) instead of its
blocking wrapper.
"""

from __future__ import annotations

import abc
import logging

from collections.abc import AsyncIterator, Sequence
from typing import TYPE_CHECKING, Any, TypeVar, cast, overload

from .._internal.arrow_stream_utils import (
    collect_arrow_table,
    create_table_iterator,
)
from .._internal.decorators import api_telemetry, pep249
from .._internal.errorcode import ER_INVALID_VALUE
from .._internal.errorhandler import ErrorHandlerMixin
from .._internal.extras import pandas, pyarrow, requires_dependency
from ..cursor._cursor_mixin import (
    CursorMixin,
    DictRow,
    Row,
    _requires_open,
    _requires_open_cursor_not_connection,
    _with_prefetch_hook,
)
from ..cursor._immutable_cursor import ImmutableCursor
from ..cursor._query_result import _MultiStatementQueryResultState, _QueryResult
from ..cursor._query_result_waiter import QueryResultWaiter
from ..cursor._result_metadata import ResultMetadata
from ..errors import InterfaceError, ProgrammingError
from ._result_batch import AsyncResultBatch


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from ..connection import Connection

logger = logging.getLogger(__name__)

T = TypeVar("T", bound=Sequence[Any])


class AsyncSnowflakeCursorBase(CursorMixin, ErrorHandlerMixin, abc.ABC):
    """Async base cursor for database operations (PEP 249 async flavour).

    Concrete subclasses must override :pyattr:`_use_dict_result` and
    :pymeth:`fetchone`.
    """

    def __init__(self, connection: Connection) -> None:
        self._init_cursor_mixin(connection)
        self._immutable: ImmutableCursor | None = None

    @property
    def _errorhandler_cursor(self) -> AsyncSnowflakeCursorBase:
        return self

    # ------------------------------------------------------------------
    # Execution
    # ------------------------------------------------------------------

    @overload
    async def callproc(self, procname: str) -> tuple: ...

    @overload
    async def callproc(self, procname: str, args: T) -> T: ...

    @pep249
    @api_telemetry
    @_requires_open
    async def callproc(self, procname: str, args: Any = None) -> Any:
        if args is None:
            args = ()
        if isinstance(args, (str, bytes)):
            raise TypeError(f"callproc args must be a sequence (e.g. list or tuple), not {type(args).__name__}")
        if not isinstance(args, Sequence):
            raise TypeError(f"callproc args must be a sequence (e.g. list or tuple), not {type(args).__name__}")
        command = f"CALL {procname}({self._connection.paramstyle.placeholders(len(args))})"
        await self.execute(command, args)
        return args

    @pep249
    @api_telemetry
    @_requires_open
    async def execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        num_statements: int | None = None,
        **kwargs: Any,
    ) -> AsyncSnowflakeCursorBase:
        if num_statements is not None:
            self.set_statement_parameter("MULTI_STATEMENT_COUNT", num_statements)

        await self.reset()
        return await self._execute(operation, parameters, _is_put_get, **kwargs)

    async def _execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        **kwargs: Any,
    ) -> AsyncSnowflakeCursorBase:
        if logger.isEnabledFor(logging.DEBUG):
            logger.debug("query: [%s]", self._format_query_for_log(operation))

        query, bindings = self._prepare_query(operation, parameters)

        try:
            immutable = await ImmutableCursor.execute(
                self._connection,
                query,
                bindings,
                statement_options=self._statement_parameters or None,
                use_dict_result=self._use_dict_result,
                use_numpy=bool(self._connection.config.numpy),
            )
        except ProgrammingError as exc:
            self._query_result = _QueryResult.from_programming_error(exc)
            raise

        self._adopt_immutable(immutable, query=query)
        self._rownumber = -1
        return self

    def _adopt_immutable(self, immutable: ImmutableCursor, query: str | None) -> None:
        self._immutable = immutable
        self._query_result = immutable.query_result

        ids = immutable.multi_query_ids
        if ids:
            self._multi_statement = _MultiStatementQueryResultState(
                parent_qid=None,
                child_query_ids=ids,
            )
            self._multi_statement._next_index = 1
        else:
            self._multi_statement = None

    @pep249
    @api_telemetry
    @_requires_open
    async def executemany(self, operation: str, seq_of_parameters: Sequence[Sequence[Any] | dict[str, Any]]) -> None:
        if not seq_of_parameters:
            return

        paramstyle = self._connection.paramstyle
        first_params = seq_of_parameters[0]

        if paramstyle.is_client_side() or isinstance(first_params, dict):
            await self.reset()
            total_rowcount = 0
            unknown_rowcount = False
            for params in seq_of_parameters:
                await self._execute(operation, params)
                rc = self._query_result.rowcount
                if rc is None or rc == -1:
                    unknown_rowcount = True
                elif not unknown_rowcount:
                    total_rowcount += rc
            self._query_result.rowcount = None if unknown_rowcount else total_rowcount
            return

        rows = cast(Sequence[Sequence[Any]], seq_of_parameters)
        first_len = len(first_params)
        for params in rows:
            if len(params) != first_len:
                raise InterfaceError(
                    msg=f"Bulk data size don't match. expected: {first_len}, got: {len(params)}, command: {operation}",
                    errno=ER_INVALID_VALUE,
                )

        num_columns = first_len
        transposed = [[row[col_idx] for row in rows] for col_idx in range(num_columns)]
        await self.execute(operation, transposed)

    @api_telemetry
    @_requires_open
    async def describe(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> list[ResultMetadata] | None:
        await self.reset()
        query, _bindings = self._prepare_query(operation, parameters)

        try:
            self._query_result = await ImmutableCursor.describe(self._connection, query)
        except ProgrammingError as exc:
            self._query_result = _QueryResult.from_programming_error(exc)
            raise

        if self._query_result.description:
            self._rownumber = -1

        return self._query_result.description

    # ------------------------------------------------------------------
    # Fetch
    # ------------------------------------------------------------------

    @_requires_open_cursor_not_connection
    @_with_prefetch_hook
    async def _fetchone(self) -> Row | DictRow | None:
        if self._immutable is None:
            return None
        row = await self._immutable.fetchone()
        self._rownumber = self._immutable.rownumber
        return row

    @pep249
    @abc.abstractmethod
    async def fetchone(self) -> Row | DictRow | None: ...

    @pep249
    @api_telemetry
    @_requires_open_cursor_not_connection
    @_with_prefetch_hook
    async def fetchmany(self, size: int | None = None) -> list[Any]:
        if size is None:
            size = self.arraysize

        if size < 0:
            raise ProgrammingError(
                msg=f"The number of rows is not zero or positive number: {size}", errno=ER_INVALID_VALUE
            )

        if size == 0:
            return []

        if self._immutable is None:
            return []
        rows = await self._immutable.fetchmany(size)
        self._rownumber = self._immutable.rownumber
        return rows

    @pep249
    @api_telemetry
    @_requires_open_cursor_not_connection
    @_with_prefetch_hook
    async def fetchall(self) -> list[Any]:
        if self._immutable is None:
            return []
        rows = await self._immutable.fetchall()
        self._rownumber = self._immutable.rownumber
        return rows

    # ------------------------------------------------------------------
    # Async iterator protocol
    # ------------------------------------------------------------------

    def __aiter__(self) -> AsyncSnowflakeCursorBase:
        return self

    async def __anext__(self) -> Row | DictRow:
        row = await self.fetchone()
        if row is None:
            raise StopAsyncIteration
        return row

    # ------------------------------------------------------------------
    # PEP 249 optional
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @_requires_open
    async def nextset(self) -> AsyncSnowflakeCursorBase | None:
        if self._multi_statement is None:
            return None

        query_id = self._multi_statement.advance()
        if query_id is None:
            return None

        ms = self._multi_statement
        self._multi_statement = None
        await self.reset()
        self._multi_statement = ms

        immutable = await ImmutableCursor.from_query_id(
            self._connection,
            query_id,
            use_dict_result=self._use_dict_result,
            use_numpy=bool(self._connection.config.numpy),
        )
        self._immutable = immutable
        self._query_result = immutable.query_result
        self._rownumber = -1

        return self

    # ------------------------------------------------------------------
    # Context manager
    # ------------------------------------------------------------------

    async def __aenter__(self) -> AsyncSnowflakeCursorBase:
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.close()

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    @_requires_open_cursor_not_connection
    async def reset(self, closing: bool = False) -> None:
        del self._messages[:]
        self._query_result.reset(closing=closing)
        if self._immutable is not None:
            try:
                await self._immutable.close()
            except Exception:
                logger.warning("Failed to close ImmutableCursor during reset", exc_info=True)
            self._immutable = None
        self._binding_data = None
        self._prefetch_hook = None
        self._multi_statement = None

    @pep249
    @api_telemetry
    async def close(self) -> bool | None:
        try:
            if self._closed:
                return False
            await self.reset(closing=True)
            self._closed = True
            del self._messages[:]
            return True
        except Exception:
            return None

    # ------------------------------------------------------------------
    # Arrow / Pandas
    # ------------------------------------------------------------------

    @requires_dependency(pyarrow)
    @api_telemetry
    @_requires_open
    @_with_prefetch_hook
    async def fetch_arrow_batches(
        self,
        force_microsecond_precision: bool = False,
    ) -> AsyncIterator[Table]:
        if self._immutable is None:
            return
        stream_ptr = await self._immutable.get_arrow_stream_ptr()
        iterator = create_table_iterator(
            stream_ptr=stream_ptr,
            connection=self._connection,
            number_to_decimal=self._connection.arrow_number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        )
        for batch in iterator:
            yield pyarrow.Table.from_batches([batch])

    @requires_dependency(pyarrow)
    @api_telemetry
    @_requires_open
    @_with_prefetch_hook
    async def fetch_arrow_all(
        self,
        force_return_table: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table | None:
        if self._immutable is None:
            return None
        stream_ptr = await self._immutable.get_arrow_stream_ptr()
        iterator = create_table_iterator(
            stream_ptr=stream_ptr,
            connection=self._connection,
            number_to_decimal=self._connection.arrow_number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        )
        return collect_arrow_table(
            table_iterator=iterator,
            columns_metadata=self._query_result.description,
            force_return_table=force_return_table,
        )

    @requires_dependency(pandas)
    @api_telemetry
    @_requires_open
    async def fetch_pandas_batches(self, **kwargs: Any) -> AsyncIterator[DataFrame]:
        async for table in self.fetch_arrow_batches(**kwargs):
            yield table.to_pandas()

    @requires_dependency(pandas)
    @api_telemetry
    @_requires_open
    async def fetch_pandas_all(self, **kwargs: Any) -> DataFrame:
        table: Table = await self.fetch_arrow_all(force_return_table=True, **kwargs)
        return table.to_pandas()

    # ------------------------------------------------------------------
    # Distributed fetch
    # ------------------------------------------------------------------

    @api_telemetry
    @_requires_open
    @_with_prefetch_hook
    async def get_result_batches(self) -> list[AsyncResultBatch] | None:
        if self._immutable is None:
            return None
        result_chunks = await self._immutable.get_chunks()
        if result_chunks is None:
            return None
        return AsyncResultBatch.from_chunks(
            list(result_chunks.chunks),
            self._query_result.description,
            self._connection,
            list(result_chunks.columns),
        )

    # ------------------------------------------------------------------
    # Async query support
    # ------------------------------------------------------------------

    @api_telemetry
    @_requires_open
    async def query_result(self, qid: str) -> AsyncSnowflakeCursorBase:
        await self.reset()

        immutable = await ImmutableCursor.from_async_query(
            self._connection,
            qid,
            use_dict_result=self._use_dict_result,
            use_numpy=bool(self._connection.config.numpy),
        )
        self._adopt_immutable(immutable, query=None)
        self._rownumber = -1

        return self

    @api_telemetry
    @_requires_open
    async def get_results_from_sfqid(self, sfqid: str) -> None:
        await self.reset()
        self.connection.get_query_status_throw_if_error(sfqid)
        self._query_result.sfqid = sfqid
        waiter = QueryResultWaiter(self._connection, sfqid)

        async def prefetch_hook() -> None:
            waiter.wait()
            self._prefetch_hook = None
            await self.query_result(sfqid)

        self._prefetch_hook = prefetch_hook

    @api_telemetry
    @_requires_open
    async def execute_async(
        self,
        command: str,
        params: Sequence[Any] | dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> dict[str, str | None]:
        await self.reset()
        return await self._execute_async(command, params)

    async def _execute_async(
        self, command: str, params: Sequence[Any] | dict[str, Any] | None
    ) -> dict[str, str | None]:
        query, bindings = self._prepare_query(command, params)
        query_id = await ImmutableCursor.execute_async(
            self._connection,
            query,
            bindings,
            statement_options=self._statement_parameters or None,
        )
        self._query_result = _QueryResult(sfqid=query_id)
        return {"queryId": query_id}

    @api_telemetry
    @_requires_open
    async def abort_query(self, qid: str) -> bool:
        return await ImmutableCursor.abort_query(self._connection, qid)


# ------------------------------------------------------------------
# Concrete subclasses
# ------------------------------------------------------------------


class AsyncSnowflakeCursor(AsyncSnowflakeCursorBase):
    """Async cursor returning results as tuples (default)."""

    @property
    def _use_dict_result(self) -> bool:
        return False

    @api_telemetry
    async def fetchone(self) -> Row | None:
        row = await self._fetchone()
        if not (row is None or isinstance(row, tuple)):
            raise TypeError(f"fetchone got unexpected result: {row}")
        return row

    async def fetchmany(self, size: int | None = None) -> list[Row]:
        return await super().fetchmany(size)

    async def fetchall(self) -> list[Row]:
        return await super().fetchall()


class AsyncDictCursor(AsyncSnowflakeCursorBase):
    """Async cursor returning results as dictionaries."""

    @property
    def _use_dict_result(self) -> bool:
        return True

    @api_telemetry
    async def fetchone(self) -> DictRow | None:
        row = await self._fetchone()
        if not (row is None or isinstance(row, dict)):
            raise TypeError(f"fetchone got unexpected result: {row}")
        return row

    async def fetchmany(self, size: int | None = None) -> list[DictRow]:
        return await super().fetchmany(size)

    async def fetchall(self) -> list[DictRow]:
        return await super().fetchall()
