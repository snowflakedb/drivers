"""Async PEP 249 Connection implementation."""

from __future__ import annotations

import asyncio
import platform
import warnings

from collections.abc import AsyncGenerator, Iterable
from io import StringIO
from typing import Any, cast

from ..._internal.api_client.client_api import async_core_driver
from ..._internal.config_utils import create_config_settings_from_dict
from ..._internal.connection import (
    APPLICATION_NAME,
    COMMIT_SQL,
    CURRENT_VERSION_SQL,
    ROLLBACK_SQL,
    SET_AUTOCOMMIT_SQL,
    SET_CLIENT_PREFETCH_THREADS_SQL,
    ConnectionMixin,
    clamp_client_prefetch_threads,
    requires_open,
)
from ..._internal.decorators import api_telemetry, backward_compatibility, internal_api, pep249
from ..._internal.errorcode import ER_INVALID_VALUE, ER_INVALID_WIF_SETTINGS
from ..._internal.logging import get_logger
from ..._internal.logout_config_mapping import (
    LogoutOptionKeys,
    logout_config_options_modifier,
)
from ..._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionGetInfoResponse,
    ConnectionGetQueryStatusResponse,
    ConnectionHandle,
    DatabaseHandle,
    WrapperIdentity,
)
from ..._internal.telemetry import AsyncTelemetryClient
from ..._internal.text_utils import split_statements
from ...connection_config import ConnectionConfig
from ...constants import QueryStatus
from ...errors import Error, ProgrammingError
from ...telemetry import TelemetryClient as _BackwardCompatTelemetryClient
from ...version import __version__
from ..cursor import CursorInstance, CursorType, DictCursor, SnowflakeCursor
from ._freezable_proxy import _ConnectionInfoProxy, _SessionParametersProxy


logger = get_logger(__name__)

# Message-substring matching because sf_core's ValidationCode is discarded at the FFI
# boundary today (ConfigError::Validation collapses to issues.first() in converter.rs,
# and DriverError's proto has no field carrying ValidationCode or the full issue list
# for the hard-fail path). Replace with a structured check once ValidationCode is
# propagated across the FFI boundary (SNOW-3406390).
_WIF_CONFLICT_MARKERS = (
    "was not set to WORKLOAD_IDENTITY",
    "impersonation_path is currently only supported for GCP, AWS, and AZURE",
)


class Connection(ConnectionMixin):
    """Async connection objects represent a database connection."""

    _session_parameters: _SessionParametersProxy
    _connection_info: _ConnectionInfoProxy

    # ------------------------------------------------------------------
    # Initialization
    # ------------------------------------------------------------------

    @api_telemetry
    def __init__(
        self,
        *,
        connection_name: str | None = None,
        connections_file_path: str | None = None,
        config: ConnectionConfig | None = None,
        **kwargs: Any,
    ) -> None:
        """
        Initialize configuration for a new async connection object.

        Unlike :class:`~snowflake.connector.connection.Connection`, this does not
        perform any I/O.  Call :meth:`connect`, use
        :func:`~snowflake.connector.aio.connect`, or ``async with`` to
        establish the session.
        """
        super().__init__(
            connection_name=connection_name,
            connections_file_path=connections_file_path,
            config=config,
            **kwargs,
        )

        self.db_handle: DatabaseHandle | None = None
        self.conn_handle: ConnectionHandle | None = None
        self._ever_opened = False
        self._close_lock = asyncio.Lock()
        self._session_parameters = _SessionParametersProxy(None)
        self._connection_info = _ConnectionInfoProxy(None)
        self._telemetry_client: AsyncTelemetryClient | None = None

    # ------------------------------------------------------------------
    # Connection setup
    # ------------------------------------------------------------------

    @api_telemetry
    async def connect(self) -> None:
        """Establish the connection to Snowflake via the Rust core."""
        if self.conn_handle is not None:
            return

        db_handle = (await async_core_driver.database_new()).db_handle
        await async_core_driver.database_init(db_handle=db_handle)
        conn_handle = (await async_core_driver.connection_new()).conn_handle

        options = self.config.to_proto_options(
            options_modifiers=[logout_config_options_modifier],
        )

        if options:
            response = await async_core_driver.connection_set_options(
                conn_handle=conn_handle,
                options=options,
                no_connection_details=self.config._no_connection_details,
            )
            for warning in response.warnings:
                warnings.warn(warning.message, stacklevel=2)

        session_params = self.config.session_parameters
        if session_params:
            await async_core_driver.connection_set_session_parameters(
                conn_handle=conn_handle,
                parameters=session_params,
            )

        try:
            await async_core_driver.connection_init(
                conn_handle=conn_handle,
                db_handle=db_handle,
                wrapper_identity=WrapperIdentity(
                    driver_name=APPLICATION_NAME,
                    driver_version=__version__,
                    language_runtime=platform.python_implementation(),
                    language_version=platform.python_version(),
                    language_compiler=platform.python_compiler(),
                ),
            )
        except ProgrammingError as e:
            # The WIF cross-param guards fire in sf_core only via connection_init
            # (ConnectionConfig::build -> validate_settings), surfaced as errno
            # ER_INVALID_VALUE. Re-map to ER_INVALID_WIF_SETTINGS for legacy parity.
            if any(marker in str(e) for marker in _WIF_CONFLICT_MARKERS):
                raise ProgrammingError(msg=str(e), errno=ER_INVALID_WIF_SETTINGS) from e
            raise

        self.conn_handle = conn_handle
        self.db_handle = db_handle
        self._ever_opened = True
        self._session_parameters = _SessionParametersProxy(conn_handle)
        self._connection_info = _ConnectionInfoProxy(conn_handle)
        self._telemetry_client = AsyncTelemetryClient(conn_handle)

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    async def close(self, retry: bool = True) -> None:
        """Close the connection, send logout, and release handles."""
        if self.conn_handle is None or await self.is_closed():
            return

        session_parameters = self._session_parameters
        connection_info = self._connection_info
        await session_parameters.freeze()
        await connection_info.freeze()

        async with self._close_lock:
            del self._messages[:]
            conn_handle, self.conn_handle = self.conn_handle, None
            db_handle, self.db_handle = self.db_handle, None

        try:
            if conn_handle and not retry:
                await async_core_driver.connection_set_options(
                    conn_handle=conn_handle,
                    options=create_config_settings_from_dict({LogoutOptionKeys.LOGOUT_MAX_ATTEMPTS: 1}),
                )
            if conn_handle:
                await async_core_driver.connection_close(conn_handle=conn_handle)
        finally:
            if conn_handle:
                await self._release_connection_handle(conn_handle)
            if db_handle:
                await self._release_database_handle(db_handle)

    async def _release_connection_handle(self, conn_handle: ConnectionHandle) -> None:
        try:
            await async_core_driver.connection_release(conn_handle=conn_handle)
        except Exception:
            logger.warning("Failed to release connection handle", exc_info=True)

    async def _release_database_handle(self, db_handle: DatabaseHandle) -> None:
        try:
            await async_core_driver.database_release(db_handle=db_handle)
        except Exception:
            logger.warning("Failed to release database handle", exc_info=True)

    # ------------------------------------------------------------------
    # Transactions
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    async def commit(self) -> None:
        cur = await self.cursor()
        try:
            await cur.execute(COMMIT_SQL)
        finally:
            await cur.close()

    @pep249
    @api_telemetry
    @requires_open
    async def rollback(self) -> None:
        cur = await self.cursor()
        try:
            await cur.execute(ROLLBACK_SQL)
        finally:
            await cur.close()

    # ------------------------------------------------------------------
    # Cursors
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    async def cursor(self, cursor_class: CursorType = SnowflakeCursor) -> CursorInstance:
        return cursor_class(self)

    # ------------------------------------------------------------------
    # Context manager
    # ------------------------------------------------------------------

    async def __aenter__(self) -> Connection:
        await self.connect()
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        try:
            if not await self.is_closed() and not await self._autocommit_enabled():
                if exc_type is None:
                    await self.commit()
                else:
                    try:
                        await self.rollback()
                    except Exception:
                        logger.warning("Rollback failed during exception handling", exc_info=True)
        finally:
            await self.close()

    # ------------------------------------------------------------------
    # Autocommit
    # ------------------------------------------------------------------

    async def _autocommit_enabled(self) -> bool:
        value = self._session_parameters["AUTOCOMMIT"]
        return value is not None and value.lower() == "true"

    @requires_open
    @api_telemetry
    async def set_autocommit(self, autocommit: bool) -> None:
        if not isinstance(autocommit, bool):
            raise ProgrammingError(msg=f"Invalid autocommit parameter: {autocommit!r}", errno=ER_INVALID_VALUE)
        cur = await self.cursor()
        try:
            await cur.execute(SET_AUTOCOMMIT_SQL.format(autocommit=str(autocommit).lower()))
        except Error as e:
            logger.warning("Autocommit feature is not enabled for this connection. Ignored: %s", e)
        finally:
            await cur.close()

    @api_telemetry
    async def get_autocommit(self) -> bool:
        return await self._autocommit_enabled()

    @pep249
    @api_telemetry
    async def autocommit(self, value: bool) -> None:
        await self.set_autocommit(value)

    # ------------------------------------------------------------------
    # Client prefetch threads
    # ------------------------------------------------------------------

    @requires_open
    @api_telemetry
    async def set_client_prefetch_threads(self, value: int) -> None:
        """Set the number of concurrent chunk-prefetch threads.

        Executes ``ALTER SESSION SET CLIENT_PREFETCH_THREADS`` so the change
        takes effect on subsequent result-set fetches. The inherited
        ``client_prefetch_threads`` property setter cannot do this itself —
        a synchronous property setter can't ``await`` — so call this method
        directly for the same immediate effect the legacy connector's setter
        has.
        """
        value = clamp_client_prefetch_threads(value)
        self.config.client_prefetch_threads = value
        cur = await self.cursor()
        try:
            await cur.execute(SET_CLIENT_PREFETCH_THREADS_SQL.format(value=value))
        finally:
            await cur.close()

    @api_telemetry
    async def get_client_prefetch_threads(self) -> int:
        """Get the configured number of chunk-prefetch threads."""
        return cast(int, self.config.client_prefetch_threads)

    # ------------------------------------------------------------------
    # Connection state
    # ------------------------------------------------------------------

    @api_telemetry
    async def is_closed(self) -> bool:
        if self.conn_handle is None:
            return self._ever_opened
        try:
            response = await async_core_driver.connection_is_closed(conn_handle=self.conn_handle)
            return bool(response.is_closed)
        except Exception:
            return True

    @api_telemetry
    async def is_expired(self) -> bool:
        """
        Return True if the connection's master token has expired.

        Once True, the session can no longer be renewed and the connection
        must be replaced; full re-authentication is required.

        Set when the server returns GS code 390114, or when a time-based check
        confirms master-token expiry just before a refresh attempt.

        Matches the legacy snowflake-connector-python ``SnowflakeConnection.expired``
        flag — intended as a read-only signal for external pool / application code.

        Unlike the sync ``Connection.expired`` property, this is a coroutine
        because the async client requires ``await`` for all RPC calls.
        """
        if self.conn_handle is None:
            return False
        try:
            response = await async_core_driver.connection_is_expired(conn_handle=self.conn_handle)
            return bool(response.is_expired)
        except Exception:
            return True

    async def is_valid(self) -> bool:
        if self.conn_handle is None or await self.is_closed():
            return False
        try:
            response = await async_core_driver.connection_heartbeat(conn_handle=self.conn_handle)
            return bool(response.valid)
        except Exception:
            return False

    # ------------------------------------------------------------------
    # Multi-statement execution
    # ------------------------------------------------------------------

    @api_telemetry
    async def execute_string(
        self,
        sql_text: str,
        remove_comments: bool = False,
        return_cursors: bool = True,
        cursor_class: CursorType = SnowflakeCursor,
        **kwargs: Any,
    ) -> Iterable[CursorInstance]:
        stream = StringIO(sql_text)
        stream_generator = self.execute_stream(stream, remove_comments=remove_comments, cursor_class=cursor_class)
        if return_cursors:
            return [cursor async for cursor in stream_generator]
        async for _ in stream_generator:
            pass
        return []

    @api_telemetry
    async def execute_stream(
        self,
        stream: StringIO,
        remove_comments: bool = False,
        cursor_class: CursorType = SnowflakeCursor,
        **kwargs: Any,
    ) -> AsyncGenerator[CursorInstance, None]:
        for sql, is_put_or_get in split_statements(stream, remove_comments=remove_comments):
            if not sql:
                continue
            cur = await self.cursor(cursor_class=cursor_class)
            await cur.execute(sql, _is_put_get=is_put_or_get)
            yield cur

    # ------------------------------------------------------------------
    # Internal API
    # ------------------------------------------------------------------

    @internal_api
    async def _get_connection_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        return await async_core_driver.connection_get_info(
            conn_handle=self.conn_handle,  # type: ignore[arg-type]
            include_master_token=include_master_token,
        )

    @internal_api
    @backward_compatibility
    def _telemetry(self) -> _BackwardCompatTelemetryClient:
        raise NotImplementedError("async _BackwardCompatTelemetryClient is not yet implemented")

    # ------------------------------------------------------------------
    # Session info
    # ------------------------------------------------------------------

    @api_telemetry
    def fetch_info(self, field: str) -> Any:
        """Fetch a single connection-info field by name."""
        return self._connection_info[field]

    @property
    def _errorhandler_connection(self) -> Connection:
        return self

    # ------------------------------------------------------------------
    # Server metadata
    # ------------------------------------------------------------------

    @api_telemetry
    async def snowflake_version(self) -> str:
        cur = await self.cursor(DictCursor)
        async with cur:
            await cur.execute(CURRENT_VERSION_SQL)
            row: dict[str, Any] = await cur.fetchone()  # type: ignore[assignment]
        return str(row["VERSION"]).split(" ")[0]

    # ------------------------------------------------------------------
    # Query status
    # ------------------------------------------------------------------

    @api_telemetry
    async def get_query_status(self, sf_qid: str) -> QueryStatus:
        status, _ = await self._get_query_status_with_response(sf_qid)
        return status

    @api_telemetry
    async def get_query_status_throw_if_error(self, sf_qid: str) -> QueryStatus:
        status, response = await self._get_query_status_with_response(sf_qid)
        if self.is_an_error(status):
            message = response.error_message if response.HasField("error_message") else f"Query {sf_qid} failed"
            errno = response.error_code if response.HasField("error_code") else -1
            raise ProgrammingError(msg=message, errno=errno, sfqid=sf_qid)
        return status

    async def _get_query_status_with_response(
        self, sf_qid: str
    ) -> tuple[QueryStatus, ConnectionGetQueryStatusResponse]:
        if self.conn_handle is None or await self.is_closed():
            return QueryStatus.DISCONNECTED, ConnectionGetQueryStatusResponse()
        response = await async_core_driver.connection_get_query_status(
            conn_handle=self.conn_handle,
            query_id=sf_qid,
        )
        try:
            status = QueryStatus[response.status_name]
        except KeyError:
            logger.warning("Unknown query status %r; treating as NO_DATA", response.status_name)
            status = QueryStatus.NO_DATA
        return status, response


SnowflakeConnection = Connection
