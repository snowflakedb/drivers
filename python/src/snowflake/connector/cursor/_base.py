"""Base cursor class for database operations (PEP 249)."""

from __future__ import annotations

import abc
import logging

from collections.abc import Callable, Iterator, Sequence
from typing import TYPE_CHECKING, Any, overload

from .._internal.api_client.client_api import core_driver
from .._internal.arrow_stream_utils import (
    collect_arrow_table,
    create_row_iterator,
    create_table_iterator,
)
from .._internal.binding_converters import ParamStyle
from .._internal.cursor import (
    Args,
    CursorBaseMixin,
    DictRow,
    MultiStatementQueryResultState,
    QueryResult,
    QueryResultWaiter,
    ResultMetadata,
    Row,
)
from .._internal.cursor.decorators import (
    requires_open,
    requires_open_cursor_not_connection,
    with_prefetch_hook,
)
from .._internal.decorators import api_telemetry, pep249
from .._internal.errorcode import ER_INVALID_VALUE
from .._internal.extras import pandas, pyarrow, requires_dependency
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    ExecuteQueryResponse,
    MultiStatementResult,
    PrepareResult,
    QueryBindings,
    ResultSetResponse,
    StatementHandle,
)
from .._internal.statement_utils import statement
from ..errors import NotSupportedError, ProgrammingError
from ..result_batch import ResultBatch
from ._result_set_wrapper import _ResultSetWrapper


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from .._internal.arrow_stream_iterator import ArrowStreamIterator
    from ..connection import Connection

logger = logging.getLogger(__name__)


def _resolve_alias(
    canonical: object,
    alias: object,
    canonical_name: str,
    alias_name: str,
) -> object:
    """Return the resolved value from a canonical/legacy-alias pair.

    Raises ProgrammingError if both are provided.
    """
    if canonical is not None and alias is not None:
        raise ProgrammingError(
            msg=f"Cannot supply both '{canonical_name}' and '{alias_name}'; pass one only.",
            errno=ER_INVALID_VALUE,
        )
    return alias if alias is not None else canonical


class SnowflakeCursorBase(CursorBaseMixin, abc.ABC):
    """
    Base cursor class for database operations (PEP 249).

    This is the abstract base for all cursor types, equivalent to
    ``SnowflakeCursorBase`` in the old connector. Concrete subclasses
    must override :pyattr:`_use_dict_result` and :pymeth:`fetchone`.
    """

    _connection: Connection
    _iterator: ArrowStreamIterator | None

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
        self._prefetch_hook: Callable[[], None] | None = None

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
    @pep249
    def connection(self) -> Connection:
        """The :class:`Connection` object that created this cursor."""
        return self._connection

    # ------------------------------------------------------------------
    # Result format control
    # ------------------------------------------------------------------

    @property
    @abc.abstractmethod
    def _use_dict_result(self) -> bool:
        """Whether fetch methods return dicts instead of tuples."""

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

    @overload
    def callproc(self, procname: str) -> tuple: ...

    @overload
    def callproc(self, procname: str, args: Args) -> Args: ...

    @pep249
    @api_telemetry
    @requires_open
    def callproc(self, procname: str, args: Any = None) -> Any:
        """Call a stored procedure.

        Args:
            procname: The stored procedure to be called.
            args: Parameters to be passed into the stored procedure.
                  ``None`` is treated as no arguments.

        Returns:
            The input parameters.
        """
        command, args = self._prepare_call_proc_statement(procname, args)
        self.execute(command, args)
        return args

    @requires_open
    def set_statement_parameter(self, key: str, value: Any) -> None:
        """Set a sticky statement-level parameter (e.g., MULTI_STATEMENT_COUNT).

        Persists across `execute()` calls on this cursor until explicitly
        changed. For per-call kwargs (one execute, no bleed across calls),
        use the `statement_parameters` channel in `execute()` instead.

        This must be called before execute() to take effect.

        Args:
            key: Parameter name (e.g., "MULTI_STATEMENT_COUNT").
            value: Parameter value.

        Raises:
            InterfaceError: If cursor is closed.

        Example:
            cursor.set_statement_parameter("MULTI_STATEMENT_COUNT", 3)
            cursor.execute("SELECT 1; SELECT 2; SELECT 3")
        """
        # Store in cursor for application in _execute
        self._set_statement_parameter(key, value)

    @pep249
    @api_telemetry
    @requires_open
    def execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        num_statements: int | None = None,
        _skip_upload_on_content_match: bool = False,
        *,
        params: Sequence[Any] | dict[str, Any] | None = None,
        _force_qmark_paramstyle: bool = False,
        **kwargs: Any,
    ) -> SnowflakeCursorBase:
        """
        Execute a database operation (query or command).
        Resets the cursor state before the execution.

        Args:
            operation (str): SQL statement to execute
            parameters (sequence or dict): Parameters for the operation.
                For qmark/numeric paramstyle: sequence of values
                For pyformat/format paramstyle: sequence (%s) or dict (%(name)s)
                For format paramstyle: sequence (%s)
            num_statements (int, optional): Number of statements in a multistatement query.
            _skip_upload_on_content_match (bool, optional): On PUT, skip
                re-upload when the remote ``x-ms-meta-sfcdigest`` matches the
                locally-computed SHA-256. Opt-in optimization for racing
                concurrent uploaders; only meaningful with ``OVERWRITE=TRUE``.
                Underscore-prefixed for parity with the legacy
                Python-connector kwarg name.
            params: Legacy alias for ``parameters`` (kwarg-only). Cannot be
                supplied together with ``parameters``.
            _force_qmark_paramstyle: If True, bind as qmark (``?``) even when
                the connection's paramstyle is pyformat/format. Used by
                callers that emit ``?`` placeholders unconditionally.
        """
        parameters = _resolve_alias(parameters, params, "parameters", "params")  # type: ignore[assignment]

        # Per-call params: this execute() only, never persisted on the cursor.
        statement_parameters = self._collect_statement_params(
            skip_upload_on_content_match=_skip_upload_on_content_match,
        )

        if num_statements is not None:
            # TODO Create a global known parameters registry
            self.set_statement_parameter("MULTI_STATEMENT_COUNT", num_statements)

        self.reset()
        return self._execute(
            operation,
            parameters,
            _is_put_get,
            statement_parameters=statement_parameters,
            _force_qmark_paramstyle=_force_qmark_paramstyle,
            **kwargs,
        )

    def _execute(
        self,
        operation: str,
        parameters: Sequence[Any] | dict[str, Any] | None = None,
        _is_put_get: bool | None = None,
        *,
        statement_parameters: dict[str, Any] | None = None,
        _force_qmark_paramstyle: bool = False,
        **kwargs: Any,
    ) -> SnowflakeCursorBase:
        """Execute query logic."""
        if logger.isEnabledFor(logging.DEBUG):
            logger.debug("query: [%s]", self._format_query_for_log(operation))

        query, bindings = self._prepare_query(operation, parameters, _force_qmark_paramstyle=_force_qmark_paramstyle)

        with statement(self.connection.conn_handle, query) as stmt_handle:  # type: ignore[arg-type]
            self._apply_statement_parameters(stmt_handle, statement_parameters)

            response = self._execute_query(stmt_handle, bindings)

            if response.HasField("multi"):
                self._handle_multi_statement_response(response.multi, query)
            else:
                self._apply_result_set(response.single, query)

        self._rownumber = -1  # reset the rownumber (rownumber is not reset in reset() for backward compatibility)
        return self

    def _apply_statement_parameters(
        self,
        stmt_handle: StatementHandle,
        statement_parameters: dict[str, Any] | None = None,
    ) -> None:
        """Apply sticky `_statement_parameters` merged with per-call
        `statement_parameters` via SetOptions RPC. Per-call wins on key
        collision and is never persisted on the cursor.
        """
        options = self._build_statement_parameters_options(statement_parameters)
        if not options:
            return
        core_driver.statement_set_options(stmt_handle=stmt_handle, options=options)

    def _execute_query(self, stmt_handle: StatementHandle, bindings: QueryBindings | None) -> ExecuteQueryResponse:
        """Execute query and return ExecuteQueryResponse (single or multi)."""
        try:
            return core_driver.statement_execute_query(stmt_handle=stmt_handle, bindings=bindings)
        except ProgrammingError as exc:
            self._query_result = QueryResult.from_programming_error(exc)
            raise

    def _handle_multi_statement_response(self, result: MultiStatementResult, query: str) -> None:
        self._multi_statement = MultiStatementQueryResultState.from_result(result)

        # Edge case: empty multi-statement result
        if self._multi_statement is None:
            self._query_result = QueryResult(query=query)
            return

        first_qid = self._multi_statement.advance()  # always non-None: from_result() guarantees non-empty children
        # already populate cursor with first child query results
        rs_response = self._fetch_result_set_by_query_id(first_qid)  # type: ignore[arg-type]
        self._apply_result_set(rs_response, query)

    def _apply_result_set(self, rs_response: ResultSetResponse, query: str | None) -> None:
        self._result_set.replace(rs_response.result_set_handle)
        self._query_result = QueryResult.from_result_set_response(rs_response, query)

    def _fetch_result_set_by_query_id(self, query_id: str) -> ResultSetResponse:
        """Fetch a ResultSetResponse (handle + descriptor) for a given query ID."""
        try:
            return core_driver.connection_get_result_set(
                conn_handle=self._connection.conn_handle,  # type: ignore[arg-type]
                query_id=query_id,
            )
        except Exception as exc:
            if isinstance(exc, ProgrammingError):
                raise
            raise ProgrammingError(msg=f"Failed to fetch result set for query_id={query_id}: {exc}") from exc

    def _prepare(self, stmt_handle: StatementHandle) -> PrepareResult | None:
        try:
            return core_driver.statement_prepare(stmt_handle=stmt_handle).result
        except ProgrammingError as exc:
            self._query_result = QueryResult.from_programming_error(exc)
            raise

    @pep249
    @api_telemetry
    @requires_open
    def executemany(
        self,
        operation: str,
        seq_of_parameters: Sequence[Sequence[Any] | dict[str, Any]] | None = None,
        *,
        seqparams: Sequence[Sequence[Any] | dict[str, Any]] | None = None,
        _force_qmark_paramstyle: bool = False,
    ) -> None:
        """
        Execute a database operation repeatedly for each element in seq_of_parameters.

        For qmark/numeric paramstyles, uses array binding to execute all parameter
        sets in a single request. For pyformat/format paramstyles, executes each
        row individually with client-side interpolation.

        Args:
            operation (str): SQL statement (typically INSERT, UPDATE, or DELETE)
            seq_of_parameters (sequence): Sequence of parameter sequences or dicts
            seqparams: Legacy alias for ``seq_of_parameters`` (kwarg-only).
                Cannot be supplied together with ``seq_of_parameters``.
            _force_qmark_paramstyle: If True, treat as qmark even when the
                connection's paramstyle is pyformat/format.

        Raises:
            InterfaceError: If parameter sequences have inconsistent lengths
        """
        seq_of_parameters = _resolve_alias(  # type: ignore[assignment]
            seq_of_parameters, seqparams, "seq_of_parameters", "seqparams"
        )

        if not seq_of_parameters:
            return  # Empty sequence - no-op per PEP 249

        paramstyle = ParamStyle.QMARK if _force_qmark_paramstyle else self._connection.paramstyle
        first_params = seq_of_parameters[0]

        # Execute individually for:
        # - Client-side binding (pyformat/format)
        # - Dict parameters (server-side doesn't support named binding)
        if paramstyle.is_client_side() or isinstance(first_params, dict):
            self.reset()
            total_rowcount = 0
            unknown_rowcount = False
            for params in seq_of_parameters:
                self._execute(
                    operation,
                    params,
                    _force_qmark_paramstyle=_force_qmark_paramstyle,
                )  # no reset between calls
                rc = self._query_result.rowcount
                if rc is None or rc == -1:
                    unknown_rowcount = True
                elif not unknown_rowcount:
                    total_rowcount += rc
            # Per PEP 249, -1 indicates that the number of rows is unknown,
            # but for backward compatibility it's set to None.
            self._query_result.rowcount = None if unknown_rowcount else total_rowcount
            return

        # Server-side binding: validate and transpose to array-binding params.
        transposed = self._build_array_binding_params(operation, seq_of_parameters, first_params)

        # Execute using array binding (existing path handles list values)
        self.execute(operation, transposed, _force_qmark_paramstyle=_force_qmark_paramstyle)

    @api_telemetry
    @requires_open
    def describe(
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
        query, bindings = self._prepare_query(operation, parameters)

        prepare_result: PrepareResult | None = None
        with statement(self.connection.conn_handle, query) as stmt_handle:  # type: ignore[arg-type]
            prepare_result = self._prepare(stmt_handle)

        self._query_result = QueryResult.from_prepare_result(prepare_result)

        if self._query_result.description:
            self._rownumber = -1

        return self._query_result.description

    # ------------------------------------------------------------------
    # Fetch – shared implementation
    # ------------------------------------------------------------------

    @requires_open_cursor_not_connection
    @with_prefetch_hook
    def _fetchone(self) -> Row | DictRow | None:
        """Fetch the next row internally.

        Return a dict if ``_use_dict_result`` is True, otherwise a tuple.
        Concrete subclasses expose this through a type-safe ``fetchone``.
        """
        if not self._iterator:
            self._iterator = self._create_row_iterator()
        try:
            row: Row | DictRow = next(self._iterator)
            self._rownumber += 1
            return row
        except StopIteration:
            return None

    @pep249
    @abc.abstractmethod
    def fetchone(self) -> Row | DictRow | None:
        """Fetch the next row of a query result set."""

    @pep249
    @api_telemetry
    @requires_open_cursor_not_connection
    @with_prefetch_hook
    def fetchmany(self, size: int | None = None) -> list[Any]:
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
            self._iterator = self._create_row_iterator()
        rows = self._iterator.fetch_many(size)
        self._rownumber += len(rows)
        return rows

    @pep249
    @api_telemetry
    @requires_open_cursor_not_connection
    @with_prefetch_hook
    def fetchall(self) -> list[Any]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            sequence: List of all remaining rows
        """
        if not self._iterator:
            self._iterator = self._create_row_iterator()
        rows = self._iterator.fetch_all()
        self._rownumber += len(rows)
        return rows

    # ------------------------------------------------------------------
    # Iterator protocol
    # ------------------------------------------------------------------

    def _create_row_iterator(self) -> ArrowStreamIterator:
        stream_ptr = self._result_set.get_arrow_stream_ptr()
        return create_row_iterator(
            stream_ptr=stream_ptr,
            connection=self._connection,
            use_dict_result=self._use_dict_result,
            use_numpy=bool(self._connection.config.numpy),
        )

    @pep249
    def __iter__(self) -> SnowflakeCursorBase:
        """
        Return the cursor itself as an iterator.

        Returns:
            SnowflakeCursorBase: Self
        """
        return self

    def __next__(self) -> Row | DictRow:
        """
        Fetch the next row from the currently executed statement.

        Returns:
            sequence: Next row

        Raises:
            StopIteration: When no more rows are available
        """
        row = self.fetchone()
        if row is None:
            raise StopIteration
        return row

    @pep249
    def next(self) -> Row | DictRow:
        """Python 2 compatibility method."""
        return self.__next__()

    # ------------------------------------------------------------------
    # PEP 249 optional / no-op methods
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    def nextset(self) -> SnowflakeCursorBase | None:
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
            cursor.execute("SELECT 1; SELECT 2; SELECT 3")
            print(cursor.fetchone())  # (1,)
            cursor.nextset()
            print(cursor.fetchone())  # (2,)
            cursor.nextset()
            print(cursor.fetchone())  # (3,)
            result = cursor.nextset()  # None - no more results
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

        rs_response = self._fetch_result_set_by_query_id(query_id)
        self._apply_result_set(rs_response, query=None)
        self._rownumber = -1

        return self

    @pep249
    @api_telemetry
    def scroll(self, value: int, mode: str = "relative") -> None:
        """Scroll the cursor in the result set."""
        raise NotSupportedError("scroll is not supported")

    # ------------------------------------------------------------------
    # Context manager
    # ------------------------------------------------------------------

    def __enter__(self) -> SnowflakeCursorBase:
        """
        Enter the runtime context for the cursor.

        Returns:
            SnowflakeCursorBase: Self
        """
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit the runtime context for the cursor."""
        self.close()

    def is_closed(self) -> bool:
        """
        Check if the cursor is closed.

        Returns:
            bool: True if closed, False otherwise
        """
        return self._closed or self._connection.is_closed()

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

    @pep249
    @api_telemetry
    def close(self) -> bool | None:
        """Close the cursor now.

        Returns whether the cursor was closed during this call.
        """
        try:
            if self._closed:
                return False
            self.reset(closing=True)
            self._closed = True
            del self._messages[:]
            return True
        except Exception:
            return None

    # ------------------------------------------------------------------
    # Fetch – Arrow / Pandas
    # ------------------------------------------------------------------

    @requires_dependency(pyarrow)
    @api_telemetry
    @requires_open
    @with_prefetch_hook
    def fetch_arrow_batches(
        self,
        force_microsecond_precision: bool = False,
    ) -> Iterator[Table]:
        """Fetch Arrow Tables in batches."""
        stream_ptr = self._result_set.get_arrow_stream_ptr()
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
    @requires_open
    @with_prefetch_hook
    def fetch_arrow_all(
        self,
        force_return_table: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table | None:
        """Fetch all results as a single Arrow Table."""
        stream_ptr = self._result_set.get_arrow_stream_ptr()
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
    @requires_open
    def fetch_pandas_batches(self, **kwargs: Any) -> Iterator[DataFrame]:
        """Fetch Pandas DataFrames in batches."""
        for table in self.fetch_arrow_batches(**kwargs):
            yield table.to_pandas()

    @requires_dependency(pandas)
    @api_telemetry
    @requires_open
    def fetch_pandas_all(self, **kwargs: Any) -> DataFrame:
        """Fetch all results as a single Pandas DataFrame."""
        table: Table = self.fetch_arrow_all(force_return_table=True, **kwargs)
        return table.to_pandas()

    # ------------------------------------------------------------------
    # Distributed fetch
    # ------------------------------------------------------------------

    @api_telemetry
    @requires_open
    @with_prefetch_hook
    def get_result_batches(self) -> list[ResultBatch] | None:
        """Get the previously executed query's ResultBatches if available."""
        result_chunks = self._result_set.get_chunks()
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
    def query_result(self, qid: str) -> SnowflakeCursorBase:
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

        response = core_driver.connection_get_query_result(
            conn_handle=self._connection.conn_handle,  # type: ignore[arg-type]
            query_id=qid,
        )

        # Handle single or multi-statement response
        if response.HasField("multi"):
            multi_result = response.multi
            if multi_result.query_ids:
                first_qid = multi_result.query_ids[0]
                rs_response = self._fetch_result_set_by_query_id(first_qid)
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
    def get_results_from_sfqid(self, sfqid: str) -> None:
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
        self.connection.get_query_status_throw_if_error(sfqid)
        self._query_result.sfqid = sfqid
        waiter = QueryResultWaiter(self._connection, sfqid)

        def prefetch_hook() -> None:
            waiter.wait()
            self._prefetch_hook = None
            self.query_result(sfqid)

        self._prefetch_hook = prefetch_hook

    @api_telemetry
    @requires_open
    def execute_async(
        self,
        command: str,
        params: Sequence[Any] | dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> dict[str, str | None]:
        """Submit a query for async execution and return immediately with the query ID.

        This is the first step in the async query lifecycle::

            # 1. Submit the query
            result = cursor.execute_async("SELECT ...")
            query_id = result["queryId"]

            # 2. Poll until complete
            status = connection.get_query_status(query_id)

            # 3. Retrieve results
            cursor.get_results_from_sfqid(query_id)

        Args:
            command: SQL statement to execute.
            params: Parameters for the operation (sequence or dict).
            **kwargs: Unused, accepted for backward compatibility.

        Returns:
            dict with a ``queryId`` key containing the Snowflake Query ID.
        """
        # TODO: deprecate returning the dict, return just the sfqid itself
        self.reset()
        return self._execute_async(command, params)

    def _execute_async(self, command: str, params: Sequence[Any] | dict[str, Any] | None) -> dict[str, str | None]:
        query, bindings = self._prepare_query(command, params)

        response = None
        with statement(self._connection.conn_handle, query) as stmt_handle:  # type: ignore[arg-type]
            response = core_driver.statement_execute_async(stmt_handle=stmt_handle, bindings=bindings)

        query_id = (response.query_id if response.query_id else None) if response else None
        self._query_result = QueryResult(sfqid=query_id)

        return {"queryId": query_id}

    @api_telemetry
    @requires_open
    def abort_query(self, qid: str) -> bool:
        """Abort a running query."""
        response = core_driver.connection_abort_query(
            conn_handle=self._connection.conn_handle,  # type: ignore[arg-type]
            query_id=qid,
        )
        return response.success
