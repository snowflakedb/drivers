"""PEP 249 Connection implementation."""

from __future__ import annotations

import atexit
import logging
import platform
import threading
import warnings

from collections.abc import Generator, Iterable
from functools import cached_property
from io import StringIO
from typing import Any, cast

from .._internal.api_client.client_api import core_driver
from .._internal.config_utils import create_config_settings_from_dict
from .._internal.connection import (
    APPLICATION_NAME,
    COMMIT_SQL,
    CURRENT_VERSION_SQL,
    ROLLBACK_SQL,
    SET_AUTOCOMMIT_SQL,
    ConnectionMixin,
    requires_open,
)
from .._internal.decorators import api_telemetry, backward_compatibility, internal_api, pep249, snowpark_compat
from .._internal.errorcode import ER_INVALID_VALUE
from .._internal.logging import safe_log
from .._internal.logout_config_mapping import (
    LogoutOptionKeys,
    logout_config_options_modifier,
)
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
    WrapperIdentity,
)
from .._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionGetInfoResponse,
    ConnectionGetQueryStatusResponse,
)
from .._internal.snowflake_restful import SnowflakeRestful
from .._internal.telemetry import TelemetryClient as _InternalTelemetryClient
from .._internal.text_utils import split_statements
from ..connection_config import ConnectionConfig
from ..constants import QueryStatus
from ..cursor import CursorInstance, CursorType, DictCursor, SnowflakeCursor
from ..errors import Error, NotSupportedError, ProgrammingError
from ..telemetry import TelemetryClient as _BackwardCompatTelemetryClient
from ..version import __version__
from ._freezable_proxy import ConnectionInfoProxy, SessionParametersProxy


logger = logging.getLogger(__name__)


class Connection(ConnectionMixin):
    """Connection objects represent a database connection."""

    _session_parameters: SessionParametersProxy
    _connection_info: ConnectionInfoProxy

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
        Initialize a new connection object.

        ``connection_name``, ``connections_file_path`` and ``config`` are
        keyword-only to preserve the previous calling contract — the legacy
        ``__init__`` was ``(self, **kwargs)`` so any positional argument was
        a ``TypeError``.  Keeping these keyword-only guarantees that an
        accidental positional call fails fast instead of being silently
        bound to one of the new parameters.

        Args:
            connection_name: Named connection to load from TOML configuration files
            connections_file_path: Path to connections configuration file
            config: Pre-built ConnectionConfig object (mutually exclusive with kwargs)
            **kwargs: Additional connection parameters
        """
        super().__init__(
            connection_name=connection_name,
            connections_file_path=connections_file_path,
            config=config,
            **kwargs,
        )

        # Backward-compat: ``auto_cleanup`` is a Python-only flag controlling whether
        # ``__del__`` / atexit should auto-close a leaked connection.  The legacy
        # snowflake-connector-python driver exposed it as ``conn.auto_cleanup`` and
        # defaulted to ``True``; preserve both here.  ``self.config.auto_cleanup``
        # is ``None`` when the caller did not provide a value, which we map to
        # the legacy default ``True``.  The field is in ``_PYTHON_ONLY`` on
        # ``ConnectionConfig`` so it is never forwarded to the Rust core.
        self.auto_cleanup: bool = True if self.config.auto_cleanup is None else bool(self.config.auto_cleanup)

        # Controls whether `query % params` is applied even when params is
        # empty (e.g. {} or ()).  When True, doubled percents (`%%`) are
        self.db_handle: DatabaseHandle | None = core_driver.database_new().db_handle
        core_driver.database_init(db_handle=self.db_handle)
        self.conn_handle: ConnectionHandle | None = core_driver.connection_new().conn_handle

        # The LogoutConfig modifier re-applies the legacy Python wrapper
        # behaviour (default ``enable_server_session_keep_alive_auto_detection=True``
        # with FutureWarning, ``False+True → None`` keep-alive remap, default
        # ``logout_error_strategy=best_effort``) on top of the generated
        # ConnectionConfig fields.  See ``logout_config_options_modifier`` for
        # the full contract.
        options = self.config.to_proto_options(
            options_modifiers=[logout_config_options_modifier],
        )

        if options:
            response = core_driver.connection_set_options(
                conn_handle=self.conn_handle, options=options, no_connection_details=self.config._no_connection_details
            )
            for warning in response.warnings:
                warnings.warn(warning.message, stacklevel=2)

        # Set session parameters if provided (before connection_init)
        session_params = self.config.session_parameters
        if session_params:
            # proto field is map<string, string>; values may be int/bool, so cast.
            core_driver.connection_set_session_parameters(
                conn_handle=self.conn_handle, parameters={k: str(v) for k, v in session_params.items()}
            )

        # Initialise close-lifecycle state before ``_connect()`` so that the
        # ``__del__`` / atexit fail-safes always observe a sane object even
        # if connection_init raises.  ``_connect()`` deliberately does NOT
        # touch these — re-initialising them there would also reset the
        # close lock if ``_connect()`` were ever called more than once.
        self._closed = False
        self._close_lock = threading.Lock()

        self._connect()

        self._session_parameters = SessionParametersProxy(self.conn_handle)
        self._connection_info = ConnectionInfoProxy(self.conn_handle)

    # ------------------------------------------------------------------
    # Connection setup
    # ------------------------------------------------------------------

    def _connect(self) -> None:
        """Establish the connection to Snowflake via the Rust core."""
        core_driver.connection_init(
            conn_handle=self.conn_handle,  # type: ignore[arg-type]
            db_handle=self.db_handle,  # type: ignore[arg-type]
            wrapper_identity=WrapperIdentity(
                driver_name=APPLICATION_NAME,
                driver_version=__version__,
                language_runtime=platform.python_implementation(),
                language_version=platform.python_version(),
                language_compiler=platform.python_compiler(),
            ),
        )
        self._telemetry_client = _InternalTelemetryClient(
            conn_handle=cast(ConnectionHandle, self.conn_handle),
        )

        if self._should_auto_cleanup():
            atexit.register(self._close_at_process_exit)

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    def close(self, retry: bool = True) -> None:
        """
        Close the connection, send logout, and release handles.

        Args:
            retry: If False, overrides max_attempts to 1 (no retries) before closing.
                   If True (default), uses init-time configuration.

        Thread-safety: the lock guards only the handle swap (nanoseconds, no I/O).
        All FFI calls use local handle copies outside the lock, so concurrent
        close() calls are safe — the second caller gets None handles and skips.
        """
        atexit.unregister(self._close_at_process_exit)

        # Fast path — Core query, no lock. Core marks is_closed=true via AtomicBool
        # at the START of connection_close (before HTTP logout), so this returns True
        # even while another thread's logout is still in-flight.
        if self.is_closed():
            return

        self._session_parameters.freeze()
        self._connection_info.freeze()

        # Lock guards ONLY the handle swap — prevents concurrent double-release.
        with self._close_lock:
            del self._messages[:]
            conn_handle, self.conn_handle = self.conn_handle, None
            db_handle, self.db_handle = self.db_handle, None

        # All I/O outside the lock, using local handle copies.
        # try/finally ensures handles are always released — on success, Strict
        # failure, or set_options failure.
        try:
            if conn_handle:
                if not retry:
                    core_driver.connection_set_options(
                        conn_handle=conn_handle,
                        options=create_config_settings_from_dict({LogoutOptionKeys.LOGOUT_MAX_ATTEMPTS: 1}),
                    )

                core_driver.connection_close(conn_handle=conn_handle)
        finally:
            if conn_handle:
                self._release_connection_handle(conn_handle)
            if db_handle:
                self._release_database_handle(db_handle)

    def _try_close(self) -> None:
        """Best-effort close for __del__ and atexit — never raises."""
        try:
            if not self.is_closed():
                self.close(retry=False)
        except Exception:
            safe_log(logger, logging.DEBUG, "close() failed during cleanup", exc_info=True)

    def _should_auto_cleanup(self) -> bool:
        """Whether this connection should auto-close on GC/exit.

        Uses getattr with False as fallback (NOT the default value of auto_cleanup,
        which is True). False here is a GC fail-safe: if __del__ fires on a
        half-initialized object (exception during __init__), we must NOT attempt
        cleanup on an object whose Core handles were never created.
        """
        return getattr(self, "auto_cleanup", False)

    def __del__(self) -> None:
        if self._should_auto_cleanup():
            self._try_close()

    def _release_connection_handle(self, conn_handle: ConnectionHandle) -> None:
        """Release the Rust-side connection handle."""
        try:
            core_driver.connection_release(conn_handle=conn_handle)
        except Exception:
            logger.warning("Failed to release connection handle", exc_info=True)

    def _release_database_handle(self, db_handle: DatabaseHandle) -> None:
        """Release the Rust-side database handle."""
        try:
            core_driver.database_release(db_handle=db_handle)
        except Exception:
            logger.warning("Failed to release database handle", exc_info=True)

    def _close_at_process_exit(self) -> None:
        """
        Cleanup handler called by atexit when process exits.

        If close() was called successfully, this handler should have been unregistered
        and should NOT run. If it runs for an already-closed connection, that indicates
        a potential bug (unregister failed, race condition, or multiple registrations).

        The entire body is wrapped in try/except because during interpreter shutdown,
        any call (FFI, logging, warnings) may fail due to torn-down module state.
        """
        try:
            if self.is_closed():
                logger.debug(
                    "atexit handler ran for already-closed connection. "
                    "This may indicate atexit.unregister() failed or a race condition occurred."
                )
                return

            # Connection is leaked (not explicitly closed) — emit FutureWarning.
            # Auto-cleanup will be disabled by default in a future version (SNOW-2314152).
            try:
                warnings.warn(
                    "Connection was not explicitly closed before process exit. "
                    "Auto-cleanup at exit will be disabled by default in a future version. "
                    "Please explicitly call connection.close() or use context manager.",
                    FutureWarning,
                    stacklevel=2,
                )
            except Exception:
                pass  # Interpreter shutting down; warning emission is best-effort

            self._try_close()
        except Exception:
            safe_log(
                logger,
                logging.WARNING,
                "_close_at_process_exit failed during interpreter shutdown",
                exc_info=True,
            )

    # ------------------------------------------------------------------
    # Transactions
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    def commit(self) -> None:
        """Commit any pending transaction to the database."""
        cur = self.cursor()
        try:
            cur.execute(COMMIT_SQL)
        finally:
            cur.close()

    @pep249
    @api_telemetry
    @requires_open
    def rollback(self) -> None:
        """Roll back to the start of any pending transaction."""
        cur = self.cursor()
        try:
            cur.execute(ROLLBACK_SQL)
        finally:
            cur.close()

    # ------------------------------------------------------------------
    # Cursors
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    def cursor(self, cursor_class: CursorType = SnowflakeCursor) -> CursorInstance:
        """
        Return a new Cursor object using the connection.

        Args:
            cursor_class: The class to use for the cursor (default: SnowflakeCursor).
                          Pass DictCursor to get results as dictionaries.

        Returns:
            SnowflakeCursorBase: A new cursor object
        """
        return cursor_class(self)

    # ------------------------------------------------------------------
    # Context manager
    # ------------------------------------------------------------------

    def __enter__(self) -> Connection:
        """
        Enter the runtime context for the connection.

        Returns:
            Connection: Self
        """
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit the runtime context. Commit on success / rollback on exception if autocommit is OFF."""
        try:
            if not self.is_closed() and not self._autocommit:
                if exc_type is None:
                    self.commit()
                else:
                    try:
                        self.rollback()
                    except Exception:
                        logger.warning("Rollback failed during exception handling", exc_info=True)
        finally:
            self.close()

    # ------------------------------------------------------------------
    # Autocommit
    # ------------------------------------------------------------------

    @property
    def _autocommit(self) -> bool:
        value = self._session_parameters["AUTOCOMMIT"]
        return value is not None and value.lower() == "true"

    @requires_open
    @api_telemetry
    def set_autocommit(self, autocommit: bool) -> None:
        """Set the autocommit mode. Executes ALTER SESSION SET autocommit on the server."""
        # FIXME: set autocommit via core
        if not isinstance(autocommit, bool):
            raise ProgrammingError(msg=f"Invalid autocommit parameter: {autocommit!r}", errno=ER_INVALID_VALUE)
        cur = self.cursor()
        try:
            cur.execute(SET_AUTOCOMMIT_SQL.format(autocommit=str(autocommit).lower()))
        except Error as e:
            logger.warning("Autocommit feature is not enabled for this connection. Ignored: %s", e)
        finally:
            cur.close()

    @api_telemetry
    def get_autocommit(self) -> bool:
        """
        Get the current autocommit mode.

        Returns:
            bool: Current autocommit setting
        """
        return self._autocommit

    @pep249
    @api_telemetry
    def autocommit(self, value: bool) -> None:
        """Set autocommit mode."""
        self.set_autocommit(value)

    # ------------------------------------------------------------------
    # Connection state
    # ------------------------------------------------------------------

    @api_telemetry
    def is_closed(self) -> bool:
        """
        Check if the connection is closed.

        Queries Core's authoritative state. If the handle has been released
        (connection_release after close), the query fails — treated as closed
        since a released handle means close() already completed.
        """
        try:
            response = core_driver.connection_is_closed(conn_handle=self.conn_handle)  # type: ignore[arg-type]
            return bool(response.is_closed)
        except Exception:
            return True

    @property
    @api_telemetry
    def expired(self) -> bool:
        """
        True if the connection's master token has expired and the session can
        no longer be renewed.  The connection must be replaced with a new one;
        full re-authentication is required.

        Set when the server returns GS code 390114, or when a time-based check
        confirms master-token expiry just before a refresh attempt.

        Matches the legacy snowflake-connector-python ``SnowflakeConnection.expired``
        flag — intended as a read-only signal for external pool / application code.

        Fails closed: if the expiry check itself errors, returns True so pools
        evict the connection on uncertainty rather than reuse a possibly-dead one
        (mirrors the async ``is_expired()`` coroutine).
        """
        try:
            response = core_driver.connection_is_expired(conn_handle=self.conn_handle)  # type: ignore[arg-type]
            return bool(response.is_expired)
        except Exception:
            return True

    @api_telemetry
    def is_valid(self) -> bool:
        """Check whether the connection is still usable for sending queries.

        Validates both the network transport and the Snowflake session by sending a heartbeat to the server.
        """
        if self.is_closed():
            return False
        try:
            response = core_driver.connection_heartbeat(conn_handle=self.conn_handle)  # type: ignore[arg-type]
            return bool(response.valid)
        except Exception:
            return False

    # ------------------------------------------------------------------
    # Multi-statement execution
    # ------------------------------------------------------------------

    @api_telemetry
    def execute_string(
        self,
        sql_text: str,
        remove_comments: bool = False,
        return_cursors: bool = True,
        cursor_class: CursorType = SnowflakeCursor,
        **kwargs: Any,
    ) -> Iterable[CursorInstance]:
        """Execute a SQL text including multiple statements. This is a non-standard convenience method."""
        stream = StringIO(sql_text)
        stream_generator = self.execute_stream(stream, remove_comments=remove_comments, cursor_class=cursor_class)
        if return_cursors:
            return list(stream_generator)
        for _ in stream_generator:
            pass
        return []

    @api_telemetry
    def execute_stream(
        self,
        stream: StringIO,
        remove_comments: bool = False,
        cursor_class: CursorType = SnowflakeCursor,
        **kwargs: Any,
    ) -> Generator[CursorInstance, None, None]:
        """Execute a stream of SQL statements. This is a non-standard convenient method."""
        for sql, is_put_or_get in split_statements(stream, remove_comments=remove_comments):
            if not sql:
                continue
            cur = self.cursor(cursor_class=cursor_class)
            cur.execute(sql, _is_put_get=is_put_or_get)
            yield cur

    # ------------------------------------------------------------------
    # Internal API
    # ------------------------------------------------------------------

    @property
    @internal_api
    @backward_compatibility
    def rest(self) -> SnowflakeRestful:
        """Internal :class:`SnowflakeRestful` instance exposed for backward compatibility."""
        return SnowflakeRestful(connection=self)

    @internal_api
    def _get_connection_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        """Return connection details from Core."""
        return core_driver.connection_get_info(
            conn_handle=self.conn_handle,  # type: ignore[arg-type]
            include_master_token=include_master_token,
        )

    @property
    @internal_api
    @backward_compatibility
    def _telemetry(self) -> _BackwardCompatTelemetryClient:
        return _BackwardCompatTelemetryClient()

    @property
    def _errorhandler_connection(self) -> Connection:
        return self

    # ------------------------------------------------------------------
    # Server metadata
    # ------------------------------------------------------------------

    @cached_property
    @api_telemetry
    def snowflake_version(self) -> str:
        """The current Snowflake server version string."""
        with self.cursor(DictCursor) as cur:
            cur.execute(CURRENT_VERSION_SQL)
            row: dict[str, Any] = cur.fetchone()  # type: ignore[assignment]
        return str(row["VERSION"]).split(" ")[0]

    # ------------------------------------------------------------------
    # Query status
    # ------------------------------------------------------------------

    @api_telemetry
    def get_query_status(self, sf_qid: str) -> QueryStatus:
        """Retrieve the status of query with sf_qid."""
        status, _ = self._get_query_status_with_response(sf_qid)
        return status

    @api_telemetry
    def get_query_status_throw_if_error(self, sf_qid: str) -> QueryStatus:
        """Retrieve the status of query with sf_qid and raises an exception if the query terminated with an error."""
        status, response = self._get_query_status_with_response(sf_qid)
        if self.is_an_error(status):
            message = response.error_message if response.HasField("error_message") else f"Query {sf_qid} failed"
            errno = response.error_code if response.HasField("error_code") else -1
            raise ProgrammingError(msg=message, errno=errno, sfqid=sf_qid)
        return status

    def _get_query_status_with_response(self, sf_qid: str) -> tuple[QueryStatus, ConnectionGetQueryStatusResponse]:
        """Fetch query status from the server and map the status name to a QueryStatus enum value."""
        if self.is_closed():
            return QueryStatus.DISCONNECTED, ConnectionGetQueryStatusResponse()
        response = core_driver.connection_get_query_status(conn_handle=self.conn_handle, query_id=sf_qid)  # type: ignore[arg-type]
        try:
            status = QueryStatus[response.status_name]
        except KeyError:
            logger.warning("Unknown query status %r; treating as NO_DATA", response.status_name)
            status = QueryStatus.NO_DATA
        return status, response

    # ------------------------------------------------------------------
    # File-transfer stub (Snowpark compatibility only)
    # ------------------------------------------------------------------

    @snowpark_compat
    def upload_stream(self, *args: Any, **kwargs: Any) -> None:
        """Noop stub — Snowpark calls this on the connection for streaming PUT.

        File transfer is not yet supported by the Universal Driver.
        """
        raise NotSupportedError("upload_stream is not yet supported by the Universal Driver.")


# Backward compatibility alias
SnowflakeConnection = Connection
