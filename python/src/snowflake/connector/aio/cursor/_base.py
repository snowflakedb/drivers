"""
Async counterpart of :mod:`snowflake.connector.cursor._base`. Defines
``SnowflakeCursorBase`` for async PEP 249 cursor operations.
"""

from __future__ import annotations

import abc
import logging

from collections.abc import AsyncIterator, Awaitable, Callable, Sequence
from typing import TYPE_CHECKING, Any, BinaryIO, cast

from ..._common.extras import pandas, pyarrow, requires_dependency
from ..._internal.api_client.client_api import CHUNK_SIZE, async_core_driver
from ..._internal.arrow_context import ArrowConverterContext
from ..._internal.arrow_stream_async import (
    AsyncArrowStreamIterator,
    collect_arrow_table_async,
    to_pandas_async,
)
from ..._internal.arrow_stream_utils import create_row_iterator, create_table_iterator
from ..._internal.binding_converters import ParamStyle
from ..._internal.cursor import (
    AsyncQueryResultWaiter,
    CursorBaseMixin,
    DictRow,
    MultiStatementQueryResultState,
    QueryResult,
    ResultMetadata,
    Row,
)
from ..._internal.cursor.decorators import (
    requires_open,
    requires_open_cursor_not_connection,
    with_prefetch_hook,
)
from ..._internal.decorators import api_telemetry, pep249
from ..._internal.errorcode import ER_INVALID_VALUE
from ..._internal.logging import get_logger
from ..._internal.protobuf_gen.database_driver_v1_pb2 import (
    ABORT_QUERY_OUTCOME_ABORTED,
    ExecuteQueryResponse,
    MultiStatementResult,
    PrepareResult,
    QueryBindings,
    ResultSetResponse,
    StatementHandle,
)
from ..._internal.statement_utils import async_statement
from ..._internal.utils import _resolve_alias
from ...errors import ProgrammingError
from ..result_batch import ResultBatch
from ._result_set_wrapper import _ResultSetWrapper


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from ..connection import Connection

logger = get_logger(__name__)

# Distinguishes "stream exhausted" from a legitimate ``None`` row value.
_FETCH_DONE = object()


class SnowflakeCursorBase(CursorBaseMixin, abc.ABC):
    """
    Base cursor class for database operations (PEP 249).

    Concrete subclasses must override :pyattr:`_use_dict_result` and :pymeth:`fetchone`.
    """

    _connection: Connection
    _iterator: AsyncArrowStreamIterator | None

    def __init__(self, connection: Connection) -> None:
        """
        Initialize a new cursor object.

        Args:
            connection: Connection object that created this cursor
        """
        self._connection = connection
        super().__init__()
        self._iterator = None

        # -- ResultSet guard (set by _execute, cleared on reset) --
        self._result_set = _ResultSetWrapper()
        # Deferred result loading (set by get_results_from_sfqid, invoked on first fetch)
        self._prefetch_hook: Callable[[], Awaitable[None]] | None = None

    # ------------------------------------------------------------------
    # Result format control
    # ------------------------------------------------------------------

    @property
    @abc.abstractmethod
    def _use_dict_result(self) -> bool:
        """Whether fetch methods return dicts instead of tuples."""

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
    @pep249
    @api_telemetry
    def connection(self) -> Connection:
        """The :class:`Connection` object that created this cursor."""
        return self._connection

    # ------------------------------------------------------------------
    # Error handling
    # ------------------------------------------------------------------

    @property
    def _errorhandler_cursor(self) -> SnowflakeCursorBase:
        return self

    @property
    def _errorhandler_connection(self) -> Connection:
        return self._connection

    # ------------------------------------------------------------------
    # Execution
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    async def callproc(self, procname: str, args: Any = None) -> Any:
        """Call a stored procedure.

        Args:
            procname: The stored procedure to be called.
            args: Parameters to be passed into the stored procedure.
                  ``None`` is treated as no arguments.

        Returns:
            The input parameters.
        """
        command, args = self._prepare_call_proc_statement(procname, args)
        await self.execute(command, args)
        return args

    @pep249
    @api_telemetry
    @requires_open
    async def execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        num_statements: int | None = None,
        _skip_upload_on_content_match: bool = False,
        *,
        _force_qmark_paramstyle: bool = False,
        _statement_params: dict[str, Any] | None = None,
        file_stream: BinaryIO | None = None,
        **kwargs: Any,
    ) -> SnowflakeCursorBase:
        """
        Execute a database operation (query or command).
        Resets the cursor state before the execution.

        Args:
            operation (str): SQL statement to execute
            parameters (sequence or dict): Parameters for the operation.
                For qmark/numeric paramstyle: sequence of values
                For pyformat paramstyle: sequence (%s) or dict (%(name)s)
                For format paramstyle: sequence (%s)
            num_statements (int, optional): Number of statements in a multistatement query.
            _skip_upload_on_content_match (bool, optional): On PUT, skip
                re-upload when the remote stored digest metadata (S3
                ``x-amz-meta-sfc-digest`` / Azure ``x-ms-meta-sfcdigest`` / GCS
                ``x-goog-meta-sfc-digest``) matches the locally-computed
                SHA-256. Opt-in optimization for racing concurrent uploaders;
                only meaningful with ``OVERWRITE=TRUE``. Underscore-prefixed
                for parity with the legacy Python-connector kwarg name.
            _force_qmark_paramstyle: If True, bind as qmark (``?``) even when
                the connection's paramstyle is pyformat/format. Used by
                callers that emit ``?`` placeholders unconditionally.
            _statement_params: Extra per-statement parameters (e.g. ``QUERY_TAG``)
                sent to Snowflake with this query only. Never persisted on the
                cursor; forwarded as query-request parameters, so they tag only
                this query without mutating session state.
            file_stream: When set, ``operation`` must be a PUT statement with
                a ``file://`` path (only the basename is used as the
                destination filename). Its contents come from this stream
                instead of disk, forwarded to the core driver in chunks so
                the whole payload is never buffered in the wrapper. Non-PUT
                ``operation`` raises ProgrammingError.
        """
        # Per-call params: this execute() only, never persisted on the cursor.
        statement_parameters = self._collect_statement_params(
            skip_upload_on_content_match=_skip_upload_on_content_match,
            num_statements=num_statements,
            statement_params=_statement_params,
        )

        self.reset()
        return await self._execute(
            operation,
            parameters,
            _is_put_get,
            statement_parameters=statement_parameters,
            _force_qmark_paramstyle=_force_qmark_paramstyle,
            file_stream=file_stream,
            **kwargs,
        )

    async def _execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        *,
        statement_parameters: dict[str, Any] | None = None,
        _force_qmark_paramstyle: bool = False,
        file_stream: BinaryIO | None = None,
        **kwargs: Any,
    ) -> SnowflakeCursorBase:
        """Execute query logic."""
        if file_stream is not None:
            await self._execute_upload_stream(operation, file_stream)
            self._rownumber = -1
            return self

        if logger.is_enabled_for(logging.INFO) and self._connection.config.log_query_text:
            logger.info("query: [%s]", self._format_query_for_log(operation))

        query, binding_params = self._prepare_query(
            operation, parameters, _force_qmark_paramstyle=_force_qmark_paramstyle
        )

        async with async_statement(self.connection.conn_handle, query) as stmt_handle:  # type: ignore[arg-type]
            self._apply_statement_parameters(stmt_handle, statement_parameters)

            bindings = self._build_query_bindings(binding_params, query) if binding_params is not None else None
            response = await self._execute_query(stmt_handle, bindings)
            request_id = response.request_id or None

            if response.HasField("multi"):
                await self._handle_multi_statement_response(response.multi, query, request_id)
            else:
                self._apply_result_set(response.single, query, request_id)

        self._rownumber = -1  # reset the rownumber (rownumber is not reset in reset() for backward compatibility)
        return self

    async def _execute_upload_stream(self, query: str, file_stream: BinaryIO) -> None:
        """Chunked streaming PUT: feed *file_stream* to the core driver in bounded chunks.

        Bytes cross the RPC boundary via begin/chunk/finish, bounding wrapper
        memory to one chunk. A failure after begin aborts the session
        server-side before re-raising.
        """
        upload_handle = (
            await async_core_driver.upload_stream_begin(
                conn_handle=self._connection.conn_handle,  # type: ignore[arg-type]
                sql=query,
            )
        ).upload_handle
        try:
            while chunk := file_stream.read(CHUNK_SIZE):
                await async_core_driver.upload_stream_chunk(upload_handle, chunk)
            finish_response = await async_core_driver.upload_stream_finish(upload_handle)
        except BaseException:
            try:
                await async_core_driver.upload_stream_abort(upload_handle)
            except Exception:
                logger.debug("upload_stream_abort during cleanup failed; propagating original error", exc_info=True)
            raise
        self._apply_result_set(finish_response, query)  # type: ignore[arg-type]

    async def _execute_query(
        self, stmt_handle: StatementHandle, bindings: QueryBindings | None
    ) -> ExecuteQueryResponse:
        """Execute query and return ExecuteQueryResponse (single or multi)."""
        try:
            return await async_core_driver.statement_execute_query(stmt_handle=stmt_handle, bindings=bindings)
        except ProgrammingError as exc:
            self._query_result = QueryResult.from_programming_error(exc)
            raise

    async def _handle_multi_statement_response(
        self, result: MultiStatementResult, query: str, request_id: str | None
    ) -> None:
        self._multi_statement = MultiStatementQueryResultState.from_result(result, request_id)

        # Edge case: empty multi-statement result
        if self._multi_statement is None:
            self._query_result = QueryResult(query=query, request_id=request_id)
            return

        first_qid = self._multi_statement.advance()  # always non-None: from_result() guarantees non-empty children
        # already populate cursor with first child query results
        rs_response = await self._fetch_result_set_by_query_id(first_qid)  # type: ignore[arg-type]
        self._apply_result_set(rs_response, query, request_id)

    def _apply_result_set(
        self, rs_response: ResultSetResponse, query: str | None, request_id: str | None = None
    ) -> None:
        self._result_set.replace(rs_response.result_set_handle)
        self._query_result = QueryResult.from_result_set_response(rs_response, query, request_id)

    async def _fetch_result_set_by_query_id(self, query_id: str) -> ResultSetResponse:
        """Fetch a ResultSetResponse (handle + descriptor) for a given query ID."""
        try:
            return await async_core_driver.connection_get_result_set(
                conn_handle=self._connection.conn_handle,  # type: ignore[arg-type]
                query_id=query_id,
            )
        except Exception as exc:
            if isinstance(exc, ProgrammingError):
                raise
            raise ProgrammingError(msg=f"Failed to fetch result set for query_id={query_id}: {exc}") from exc

    async def _prepare(self, stmt_handle: StatementHandle) -> PrepareResult | None:
        try:
            return (await async_core_driver.statement_prepare(stmt_handle=stmt_handle)).result
        except ProgrammingError as exc:
            self._query_result = QueryResult.from_programming_error(exc)
            raise

    async def _executemany_per_row(
        self,
        operation: str,
        seq_of_parameters: Sequence[Sequence[Any] | dict[str, Any]],
        _force_qmark_paramstyle: bool = False,
    ) -> None:
        """Execute each parameter set individually and aggregate rowcount.

        Used by ``executemany`` whenever array binding does not apply: client-side
        paramstyles, dict parameters, or statements the server reports as not
        array-bindable (e.g. UPDATE, DELETE, MERGE).
        """
        self.reset()
        total_rowcount = 0
        unknown_rowcount = False
        for params in seq_of_parameters:
            await self._execute(
                operation, params, _force_qmark_paramstyle=_force_qmark_paramstyle
            )  # no reset between calls
            rc = self._query_result.rowcount
            if rc is None or rc == -1:
                unknown_rowcount = True
            elif not unknown_rowcount:
                total_rowcount += rc
        # Per PEP 249, -1 indicates that the number of rows is unknown,
        # but for backward compatibility it's set to None.
        self._query_result.rowcount = None if unknown_rowcount else total_rowcount

    @pep249
    @api_telemetry
    @requires_open
    async def executemany(
        self,
        operation: str,
        seq_of_parameters: Sequence[Sequence[Any] | dict[str, Any]] | None = None,
        *,
        seqparams: Sequence[Sequence[Any] | dict[str, Any]] | None = None,
        _force_qmark_paramstyle: bool = False,
    ) -> SnowflakeCursorBase:
        """
        Execute a database operation repeatedly for each element in seq_of_parameters.

        For qmark/numeric paramstyles with sequence parameters, issues a describe
        request first to read the server's ``arrayBindSupported`` flag (the same
        mechanism used by the JDBC and ODBC drivers). If supported, all rows are
        transposed into column-major arrays and sent in a single request. If not
        supported (e.g. UPDATE, DELETE, MERGE), each parameter set is executed
        individually with scalar JSON binding.

        For pyformat/format paramstyles and dict parameters, executes each row
        individually with client-side interpolation.

        Args:
            operation (str): SQL statement (INSERT, UPDATE, DELETE, etc.)
            seq_of_parameters (sequence): Sequence of parameter sequences or dicts
            seqparams: Legacy alias for ``seq_of_parameters`` (kwarg-only).
                Cannot be supplied together with ``seq_of_parameters``.
            _force_qmark_paramstyle: If True, treat as qmark even when the
                connection's paramstyle is pyformat/format.

        Returns:
            The cursor itself (``self``), so callers can read ``.sfqid`` after
            execution — matching the legacy connector.

        Raises:
            InterfaceError: If parameter sequences have inconsistent lengths
        """
        seq_of_parameters = _resolve_alias(  # type: ignore[assignment]
            seq_of_parameters, seqparams, "seq_of_parameters", "seqparams"
        )

        if not seq_of_parameters:
            return self  # Empty sequence - no-op per PEP 249

        paramstyle = ParamStyle.QMARK if _force_qmark_paramstyle else self._connection.paramstyle
        first_params = seq_of_parameters[0]

        # Execute individually for:
        # - Client-side binding (pyformat/format)
        # - Dict parameters (server-side doesn't support named binding)
        if paramstyle.is_client_side() or isinstance(first_params, dict):
            if paramstyle.is_client_side():
                # INSERT with client-side binding: rewrite into a single
                # multi-row INSERT to avoid one HTTP request per row.
                rewritten = self._rewrite_multirow_insert(operation, seq_of_parameters)
                if rewritten is not None:
                    # Values were already interpolated and escaped by
                    # ClientSideBindingConverter.interpolate_query — no further binding needed.
                    return await self.execute(rewritten)
            await self._executemany_per_row(operation, seq_of_parameters, _force_qmark_paramstyle)
            return self

        # Validate row widths and pre-compute column-major layout before the server
        # round-trip so InterfaceError from mismatched lengths surfaces early.
        transposed = self._build_array_binding_params(operation, seq_of_parameters, first_params)

        # Ask the server whether array binding is supported for this statement.
        # Mirrors JDBC (describeSqlIfNotTried) and the C/ODBC driver describe-only request.
        async with async_statement(self._connection.conn_handle, operation) as stmt_handle:  # type: ignore[arg-type]
            prepare_result = await self._prepare(stmt_handle)

        if prepare_result is None or not prepare_result.array_bind_supported:
            # Per-row fallback: server does not support array binding for this
            # statement type (e.g. UPDATE, DELETE, MERGE).
            await self._executemany_per_row(operation, seq_of_parameters, _force_qmark_paramstyle)
            return self

        # Server confirmed array binding: use pre-computed column-major params.
        return await self.execute(operation, transposed, _force_qmark_paramstyle=_force_qmark_paramstyle)

    @api_telemetry
    @requires_open
    async def describe(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> list[ResultMetadata] | None:
        """Obtain the schema of the result without executing the query.

        This method prepares the query on the server with describeOnly=true to obtain
        column metadata without actually executing the query or returning data rows.

        Args:
            operation: SQL statement to describe
            parameters: Parameters for the SQL statement (same as execute())
            **kwargs: Additional keyword arguments (for future compatibility)

        Returns:
            List of ResultMetadata tuples describing result columns, or None if the
            statement produces no result set (e.g., INSERT, UPDATE, DELETE, DDL).

        Side effects:
            - Updates cursor.description with the column metadata
        """
        self.reset()
        query, _ = self._prepare_query(operation, parameters)

        prepare_result: PrepareResult | None = None
        async with async_statement(self.connection.conn_handle, query) as stmt_handle:  # type: ignore[arg-type]
            prepare_result = await self._prepare(stmt_handle)

        self._query_result = QueryResult.from_prepare_result(prepare_result)

        if self._query_result.description:
            self._rownumber = -1

        return self._query_result.description

    # ------------------------------------------------------------------
    # Fetch – shared implementation
    # Intentionally no @api_telemetry on fetch methods - they are hot paths.
    # ------------------------------------------------------------------

    @requires_open_cursor_not_connection
    @with_prefetch_hook
    async def _fetchone(self) -> Row | DictRow | None:
        """Fetch the next row internally.

        Return a dict if ``_use_dict_result`` is True, otherwise a tuple.
        Concrete subclasses expose this through a type-safe ``fetchone``.
        """
        if not self._iterator:
            self._iterator = await self._create_row_iterator()
        row = await self._iterator.fetch_next(default=_FETCH_DONE)
        if row is _FETCH_DONE:
            return None
        self._rownumber += 1
        return cast(Row | DictRow, row)

    @pep249
    @abc.abstractmethod
    async def fetchone(self) -> Row | DictRow | None:
        """Fetch the next row of a query result set."""

    @pep249
    @requires_open_cursor_not_connection
    @with_prefetch_hook
    async def fetchmany(self, size: int | None = None) -> list[Any]:
        """
        Fetch the next set of rows of a query result.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            sequence: List of rows

        Raises:
            ProgrammingError: If the number of rows is not zero or positive number
        """
        if size is None:
            size = self.arraysize

        if size < 0:
            raise ProgrammingError(
                msg=f"The number of rows is not zero or positive number: {size}", errno=ER_INVALID_VALUE
            )

        if size == 0:
            return []

        if not self._iterator:
            self._iterator = await self._create_row_iterator()
        rows = await self._iterator.fetch_many(size)
        self._rownumber += len(rows)
        return rows

    @pep249
    @requires_open_cursor_not_connection
    @with_prefetch_hook
    async def fetchall(self) -> list[Any]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            sequence: List of all remaining rows
        """
        if not self._iterator:
            self._iterator = await self._create_row_iterator()
        rows = await self._iterator.fetch_all()
        self._rownumber += len(rows)
        return rows

    # ------------------------------------------------------------------
    # Iterator protocol
    # ------------------------------------------------------------------

    async def _create_row_iterator(self) -> AsyncArrowStreamIterator:
        stream_ptr = await self._result_set.get_arrow_stream_ptr()
        return AsyncArrowStreamIterator(
            create_row_iterator(
                stream_ptr=stream_ptr,
                context=ArrowConverterContext.create(self._connection),
                use_dict_result=self._use_dict_result,
                use_numpy=bool(self._connection.config.numpy),
            )
        )

    @pep249
    def __aiter__(self) -> SnowflakeCursorBase:
        """
        Return the cursor itself as an async iterator.

        Returns:
            SnowflakeCursorBase: Self
        """
        return self

    async def __anext__(self) -> Row | DictRow:
        """
        Fetch the next row from the currently executed statement.

        Returns:
            sequence: Next row

        Raises:
            StopAsyncIteration: When no more rows are available
        """
        row = await self.fetchone()
        if row is None:
            raise StopAsyncIteration
        return row

    @pep249
    async def next(self) -> Row | DictRow:
        """Python 2 compatibility method."""
        return await self.__anext__()

    # ------------------------------------------------------------------
    # PEP 249 optional / no-op methods
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    async def nextset(self) -> SnowflakeCursorBase | None:
        """
        Skip to the next available result set, discarding remaining rows from current set.

        This method is used for multi-statement queries where a single execute() produces
        multiple result sets. Call nextset() to advance to the next query's results.

        Returns:
            SnowflakeCursorBase: Self if next set is available.
            None: If no more result sets are available.

        Raises:
            InterfaceError: If cursor is closed.

        Example:
            cursor.set_statement_parameter("MULTI_STATEMENT_COUNT", 3)
            await cursor.execute("SELECT 1; SELECT 2; SELECT 3")
            print(await cursor.fetchone())  # (1,)
            await cursor.nextset()
            print(await cursor.fetchone())  # (2,)
            await cursor.nextset()
            print(await cursor.fetchone())  # (3,)
            result = await cursor.nextset()  # None - no more results
        """
        if self._multi_statement is None:
            return None

        query_id = self._multi_statement.advance()
        if query_id is None:
            return None

        # Detach multi-statement state so reset() doesn't clear it
        ms = self._multi_statement
        self._multi_statement = None
        self.reset()
        self._multi_statement = ms

        rs_response = await self._fetch_result_set_by_query_id(query_id)
        self._apply_result_set(rs_response, query=None, request_id=ms.request_id)
        self._rownumber = -1

        return self

    # ------------------------------------------------------------------
    # Context manager
    # ------------------------------------------------------------------

    async def __aenter__(self) -> SnowflakeCursorBase:
        """
        Enter the runtime context for the cursor.

        Returns:
            SnowflakeCursorBase: Self
        """
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit the runtime context for the cursor."""
        self.close()

    @api_telemetry
    @requires_open_cursor_not_connection
    def reset(self, closing: bool = False) -> None:
        """Reset the result set.

        Frees heavy result data (arrow streams, multi-statement state) while
        for backward compatibility preserving metadata that the old driver
        also keeps across resets: ``description``, ``rownumber``, ``sfqid``,
        ``query``, and ``sqlstate``.

        Also clears the ``messages`` list so that errors from previous
        operations do not leak into the next one.

        Args:
            closing: If True, do not reset rowcount,
                     see: SNOW-647539: Do not erase the rowcount information when closing the cursor.
                     If False, reset rowcount to None.
        """
        del self._messages[:]
        self._query_result.reset(closing=closing)
        self._result_set.release()
        self._iterator = None
        self._binding_data = None
        self._prefetch_hook = None
        # Clear multistatement state
        self._multi_statement = None

    # ------------------------------------------------------------------
    # Fetch – Arrow / Pandas
    # ------------------------------------------------------------------

    @requires_dependency(pyarrow)
    @api_telemetry
    @requires_open
    @with_prefetch_hook
    async def fetch_arrow_batches(
        self,
        force_microsecond_precision: bool = False,
    ) -> AsyncIterator[Table]:
        """Fetch Arrow Tables in batches."""
        stream_ptr = await self._result_set.get_arrow_stream_ptr()
        iterator = AsyncArrowStreamIterator(
            create_table_iterator(
                stream_ptr=stream_ptr,
                context=ArrowConverterContext.create(self._connection),
                number_to_decimal=self._connection.arrow_number_to_decimal,
                force_microsecond_precision=force_microsecond_precision,
            )
        )
        async for batch in iterator:
            yield pyarrow.Table.from_batches([batch])

    @requires_dependency(pyarrow)
    @api_telemetry
    @requires_open
    @with_prefetch_hook
    async def fetch_arrow_all(
        self,
        force_return_table: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table | None:
        """Fetch all results as a single Arrow Table."""
        stream_ptr = await self._result_set.get_arrow_stream_ptr()
        iterator = create_table_iterator(
            stream_ptr=stream_ptr,
            context=ArrowConverterContext.create(self._connection),
            number_to_decimal=self._connection.arrow_number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        )
        return await collect_arrow_table_async(
            iterator,
            columns_metadata=self._query_result.description,
            force_return_table=force_return_table,
        )

    @requires_dependency(pandas)
    @api_telemetry
    @requires_open
    async def fetch_pandas_batches(self, **kwargs: Any) -> AsyncIterator[DataFrame]:
        """Fetch Pandas DataFrames in batches.

        ``force_microsecond_precision`` (if present) governs the Arrow
        conversion; all other kwargs are forwarded to ``pyarrow.Table.to_pandas``
        (e.g. ``split_blocks``), matching snowflake-connector-python.
        """
        force_microsecond_precision = kwargs.pop("force_microsecond_precision", False)
        async for table in self.fetch_arrow_batches(force_microsecond_precision=force_microsecond_precision):
            yield await to_pandas_async(table, **kwargs)

    @requires_dependency(pandas)
    @api_telemetry
    @requires_open
    async def fetch_pandas_all(self, **kwargs: Any) -> DataFrame:
        """Fetch all results as a single Pandas DataFrame.

        ``force_microsecond_precision`` (if present) governs the Arrow
        conversion; all other kwargs are forwarded to ``pyarrow.Table.to_pandas``.
        """
        force_microsecond_precision = kwargs.pop("force_microsecond_precision", False)
        table: Table = await self.fetch_arrow_all(
            force_return_table=True, force_microsecond_precision=force_microsecond_precision
        )
        return await to_pandas_async(table, **kwargs)

    # ------------------------------------------------------------------
    # Distributed fetch
    # ------------------------------------------------------------------

    @api_telemetry
    @requires_open
    @with_prefetch_hook
    async def get_result_batches(self) -> list[ResultBatch] | None:
        """Get the previously executed query's ResultBatches if available."""
        result_chunks = await self._result_set.get_chunks()
        if result_chunks is None:
            return None
        return ResultBatch.from_chunks(
            list(result_chunks.chunks),
            self._query_result.description,
            self._connection,
            list(result_chunks.columns),
        )

    # ------------------------------------------------------------------
    # Async query support
    # ------------------------------------------------------------------

    @api_telemetry
    @requires_open
    async def query_result(self, qid: str) -> SnowflakeCursorBase:
        """
        Fetch the result of a previously executed query by its Snowflake Query ID.

        Resets the cursor and populates it with the results from the specified
        query, making them available through the standard fetch methods
        (fetchone, fetchall, fetch_arrow_all, etc.).

        Args:
            qid: Snowflake Query ID (sfqid) of the previously executed query.

        Returns:
            This cursor instance, now populated with the query results.

        Raises:
            ProgrammingError: If the query ID is invalid, the query is still
                running, or the results are no longer available.
        """
        self.reset()

        response = await async_core_driver.connection_get_query_result(
            conn_handle=self._connection.conn_handle,  # type: ignore[arg-type]
            query_id=qid,
        )

        # Handle single or multi-statement response
        if response.HasField("multi"):
            multi_result = response.multi
            if multi_result.query_ids:
                first_qid = multi_result.query_ids[0]
                rs_response = await self._fetch_result_set_by_query_id(first_qid)
                self._apply_result_set(rs_response, query=None)
            else:
                self._query_result = QueryResult()
        else:
            rs_response = response.single
            self._apply_result_set(rs_response, query=None)

        self._rownumber = -1

        return self

    @api_telemetry
    @requires_open
    async def get_results_from_sfqid(self, sfqid: str) -> None:
        """Get results from a previously executed query.

        Polls query status until completion, then loads results lazily
        on first fetch call.

        Before the first fetch triggers the prefetch hook, result-dependent
        cursor attributes and methods such as ``description``, ``rowcount``,
        and ``fetch*`` are not populated and MUST NOT be relied upon.

        Args:
            sfqid: Snowflake Query ID of the target query.

        Raises:
            ProgrammingError: If the query has already failed at call time,
                or if it reaches a terminal error status while being polled
                for completion.
            DatabaseError: If the server stops returning status information
                while polling for query completion.
        """
        self.reset()
        await self.connection.get_query_status_throw_if_error(sfqid)
        self._query_result.sfqid = sfqid
        waiter = AsyncQueryResultWaiter(self._connection, sfqid)

        async def prefetch_hook() -> None:
            await waiter.wait()
            self._prefetch_hook = None
            await self.query_result(sfqid)

        self._prefetch_hook = prefetch_hook

    @api_telemetry
    @requires_open
    async def execute_async(
        self,
        command: str,
        params: Sequence[Any] | dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> dict[str, str | None]:
        """Submit a query for async execution and return immediately with the query ID.

        This is the first step in the async query lifecycle::

            # 1. Submit the query
            result = await cursor.execute_async("SELECT ...")
            query_id = result["queryId"]

            # 2. Poll until complete
            status = connection.get_query_status(query_id)

            # 3. Retrieve results
            await cursor.get_results_from_sfqid(query_id)

        Args:
            command: SQL statement to execute.
            params: Parameters for the operation (sequence or dict).
            **kwargs: Unused, accepted for backward compatibility.

        Returns:
            dict with a ``queryId`` key containing the Snowflake Query ID.
        """
        # TODO: deprecate returning the dict, return just the sfqid itself
        self.reset()
        return await self._execute_async(command, params)

    async def _execute_async(
        self, command: str, params: Sequence[Any] | dict[str, Any] | None
    ) -> dict[str, str | None]:
        query, binding_params = self._prepare_query(command, params)

        async with async_statement(self._connection.conn_handle, query) as stmt_handle:  # type: ignore[arg-type]
            bindings = self._build_query_bindings(binding_params, query) if binding_params is not None else None
            response = await async_core_driver.statement_execute_async(stmt_handle=stmt_handle, bindings=bindings)
        query_id = response.query_id or None
        request_id = response.request_id or None
        self._query_result = QueryResult(sfqid=query_id, request_id=request_id)

        return {"queryId": query_id}

    @api_telemetry
    @requires_open
    async def abort_query(self, qid: str) -> bool:
        """Abort a running query."""
        response = await async_core_driver.connection_abort_query(
            conn_handle=self._connection.conn_handle,  # type: ignore[arg-type]
            query_id=qid,
        )
        return response.outcome == ABORT_QUERY_OUTCOME_ABORTED
