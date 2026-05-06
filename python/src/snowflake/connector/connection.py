"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

import atexit
import functools
import logging
import platform
import threading
import warnings

from collections.abc import Callable, Generator, Iterable
from functools import cached_property
from io import StringIO
from typing import Any, TypeVar, cast

from ._internal.api_client.client_api import database_driver_client
from ._internal.binding_converters import ParamStyle
from ._internal.config_utils import create_config_settings_from_dict
from ._internal.decorators import api_telemetry, backward_compatibility, internal_api, pep249
from ._internal.errorcode import ER_CONNECTION_IS_CLOSED, ER_INVALID_VALUE
from ._internal.errorhandler import ErrorHandlerMixin
from ._internal.extras import check_dependency
from ._internal.extras import numpy as np
from ._internal.freezable_proxy import ConnectionInfoProxy, SessionParametersProxy
from ._internal.logout_config_mapping import (
    LogoutOptionKeys,
    logout_config_options_modifier,
)
from ._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
    WrapperIdentity,
)
from ._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionCloseRequest,
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionGetQueryStatusRequest,
    ConnectionGetQueryStatusResponse,
    ConnectionHeartbeatRequest,
    ConnectionInitRequest,
    ConnectionIsClosedRequest,
    ConnectionNewRequest,
    ConnectionReleaseRequest,
    ConnectionSetOptionsRequest,
    ConnectionSetSessionParametersRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
    DatabaseReleaseRequest,
)
from ._internal.snowflake_restful import SnowflakeRestful
from ._internal.sqlstate import SQLSTATE_CONNECTION_NOT_EXISTS
from ._internal.text_utils import split_statements
from .connection_config import ConnectionConfig
from .constants import QueryStatus
from .cursor import CursorInstance, CursorType, DictCursor, SnowflakeCursor
from .errors import DatabaseError, Error, ErrorValue, InterfaceError, ProgrammingError
from .telemetry import TelemetryClient as _BackwardCompatTelemetryClient
from .version import __version__


# backward compatibility constant
# snowflake-sqlalchemy imports this symbol and calls .get(name) in
# parse_query_param_type to cast URL query-string values to the types the
# connector expects.  The universal driver validates parameters internally, so
# an empty dict is correct: every .get() returns None and values pass through
# uncast.
DEFAULT_CONFIGURATION: dict[str, tuple[Any, tuple[type, ...]]] = {}

_APPLICATION_NAME = "PythonConnector"
# Kept as a public alias for backward compatibility — external packages
# (e.g. snowflake-sqlalchemy) may import this symbol.
CLIENT_NAME = _APPLICATION_NAME

# Default upper bound for query strings included in log messages.  Mirrors
# the ``log_max_query_length`` default emitted by the generated
# :class:`ConnectionConfig` (sourced from ``PARAM_DEFS``); kept here as a
# named constant so the property fallback isn't a magic number.
LOG_MAX_QUERY_LENGTH = 80

SessionParameters = dict[str, Any]
ConnectionParamValue = int | str | float | bytes | bool | SessionParameters
ConnectionParameters = dict[str, ConnectionParamValue]

# Module-level logger
logger = logging.getLogger(__name__)


F = TypeVar("F", bound=Callable[..., Any])


def _requires_open(func: F) -> F:
    """Raise ``DatabaseError`` if the connection is closed (mirrors old driver behavior)."""
    # TODO: it should rather raise InterfaceError, consider to align with the cursor

    @functools.wraps(func)
    def wrapper(self: Connection, *args: Any, **kwargs: Any) -> Any:
        if self.is_closed():
            raise DatabaseError(
                msg="Connection is closed.",
                errno=ER_CONNECTION_IS_CLOSED,
                sqlstate=SQLSTATE_CONNECTION_NOT_EXISTS,
            )
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


class Connection(ErrorHandlerMixin):
    """Connection objects represent a database connection."""

    def __init__(
        self,
        *,
        connection_name: str | None = None,
        connections_file_path: str | None = None,
        config: ConnectionConfig | None = None,
        **kwargs: ConnectionParamValue,
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
        self._messages: list[tuple[type[Exception], ErrorValue]] = []
        self._errorhandler: Callable[..., None] = Error.default_errorhandler

        self.config = ConnectionConfig.from_connection_args(
            connection_name=connection_name,
            connections_file_path=connections_file_path,
            config=config,
            **kwargs,
        )

        # paramstyle (via setter so str | ParamStyle normalization is single-sourced)
        from snowflake.connector import paramstyle as default_paramstyle

        self.paramstyle = self.config.paramstyle or default_paramstyle

        # Validate numpy dependency
        if self.config.numpy:
            check_dependency(np)

        # Backward-compat: ``auto_cleanup`` is a Python-only flag controlling whether
        # ``__del__`` / atexit should auto-close a leaked connection.  The legacy
        # snowflake-connector-python driver exposed it as ``conn.auto_cleanup`` and
        # defaulted to ``True``; preserve both here.  ``self.config.auto_cleanup``
        # is ``None`` when the caller did not provide a value, which we map to
        # the legacy default ``True``.  The field is in ``_PYTHON_ONLY`` on
        # ``ConnectionConfig`` so it is never forwarded to the Rust core.
        self.auto_cleanup: bool = True if self.config.auto_cleanup is None else bool(self.config.auto_cleanup)

        self.db_api = database_driver_client()
        self.db_handle: DatabaseHandle | None = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle: ConnectionHandle | None = self.db_api.connection_new(ConnectionNewRequest()).conn_handle

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
            response = self.db_api.connection_set_options(
                ConnectionSetOptionsRequest(
                    conn_handle=self.conn_handle,
                    options=options,
                )
            )
            for warning in response.warnings:
                warnings.warn(warning.message, stacklevel=2)

        # Set session parameters if provided (before connection_init)
        session_params = self.config.session_parameters
        if session_params:
            self.db_api.connection_set_session_parameters(
                ConnectionSetSessionParametersRequest(conn_handle=self.conn_handle, parameters=session_params)
            )

        # Initialise close-lifecycle state before ``_connect()`` so that the
        # ``__del__`` / atexit fail-safes always observe a sane object even
        # if connection_init raises.  ``_connect()`` deliberately does NOT
        # touch these — re-initialising them there would also reset the
        # close lock if ``_connect()`` were ever called more than once.
        self._closed = False
        self._close_lock = threading.Lock()

        self._connect()

        self._session_parameters = SessionParametersProxy(self.db_api, self.conn_handle)
        self._connection_info = ConnectionInfoProxy(self.db_api, self.conn_handle)

        _sensitive_keys = {"password", "private_key", "passcode", "private_key_password", "private_key_file_pwd"}
        self.kwargs = {k: ("***" if k in _sensitive_keys else v) for k, v in kwargs.items()}

    def _connect(self) -> None:
        """Establish the connection to Snowflake via the Rust core."""
        self.db_api.connection_init(
            ConnectionInitRequest(
                conn_handle=self.conn_handle,
                db_handle=self.db_handle,
                wrapper_identity=WrapperIdentity(
                    driver_name=_APPLICATION_NAME,
                    driver_version=__version__,
                    language_runtime=platform.python_implementation(),
                    language_version=platform.python_version(),
                    language_compiler=platform.python_compiler(),
                ),
            )
        )
        from ._internal.telemetry import TelemetryClient

        self._telemetry_client = TelemetryClient(
            db_api=self.db_api,
            conn_handle=cast(ConnectionHandle, self.conn_handle),
        )

        if self._should_auto_cleanup():
            atexit.register(self._close_at_process_exit)

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
                    self.db_api.connection_set_options(
                        ConnectionSetOptionsRequest(
                            conn_handle=conn_handle,
                            options=create_config_settings_from_dict({LogoutOptionKeys.LOGOUT_MAX_ATTEMPTS: 1}),
                        )
                    )

                # Logout + mark closed in Core (network I/O, bounded by Core's timeout)
                self.db_api.connection_close(ConnectionCloseRequest(conn_handle=conn_handle))
        finally:
            # Release handles in Core's object store.
            # Separated from connection_close for future connection pooling.
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
            try:
                logger.debug("close() failed during cleanup")
            except Exception:
                pass

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
            self.db_api.connection_release(ConnectionReleaseRequest(conn_handle=conn_handle))
        except Exception:
            logger.warning("Failed to release connection handle", exc_info=True)

    def _release_database_handle(self, db_handle: DatabaseHandle) -> None:
        """Release the Rust-side database handle."""
        try:
            self.db_api.database_release(DatabaseReleaseRequest(db_handle=db_handle))
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
            try:
                logger.warning("_close_at_process_exit failed during interpreter shutdown")
            except Exception:
                pass  # logger itself may be torn down

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    @pep249
    @api_telemetry
    @_requires_open
    def commit(self) -> None:
        """Commit any pending transaction to the database."""
        cur = self.cursor()
        try:
            cur.execute("COMMIT")
        finally:
            cur.close()

    @pep249
    @api_telemetry
    @_requires_open
    def rollback(self) -> None:
        """Roll back to the start of any pending transaction."""
        cur = self.cursor()
        try:
            cur.execute("ROLLBACK")
        finally:
            cur.close()

    @pep249
    @api_telemetry
    @_requires_open
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

    # Context manager support
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

    @property
    def _autocommit(self) -> bool:
        value = self._get_session_parameter("AUTOCOMMIT")
        return value is not None and value.lower() == "true"

    @_requires_open
    @api_telemetry
    def set_autocommit(self, autocommit: bool) -> None:
        """Set the autocommit mode. Executes ALTER SESSION SET autocommit on the server."""
        # FIXME: set autocommit via core
        if not isinstance(autocommit, bool):
            raise ProgrammingError(msg=f"Invalid autocommit parameter: {autocommit!r}", errno=ER_INVALID_VALUE)
        cur = self.cursor()
        try:
            cur.execute(f"ALTER SESSION SET autocommit={str(autocommit).lower()}")
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
    def autocommit(self, value: bool) -> None:
        """Set autocommit mode."""
        self.set_autocommit(value)

    def is_closed(self) -> bool:
        """
        Check if the connection is closed.

        Queries Core's authoritative state. If the handle has been released
        (connection_release after close), the query fails — treated as closed
        since a released handle means close() already completed.
        """
        try:
            response = self.db_api.connection_is_closed(ConnectionIsClosedRequest(conn_handle=self.conn_handle))
            return bool(response.is_closed)
        except Exception:
            # Handle released or FFI unavailable — connection is closed
            return True

    def is_valid(self) -> bool:
        """Check whether the connection is still usable for sending queries.

        Validates both the network transport and the Snowflake session by sending a heartbeat to the server.
        """
        if self.is_closed():
            return False
        try:
            request = ConnectionHeartbeatRequest(conn_handle=self.conn_handle)
            response = self.db_api.connection_heartbeat(request)
            return response.valid
        except Exception:
            return False

    def _get_session_parameter(self, name: str) -> str | None:
        """
        Get a session parameter value (internal method).

        Args:
            name: The parameter name (case-insensitive)

        Returns:
            str | None: The parameter value, or None if not found
        """
        return self._session_parameters[name]

    @property
    def paramstyle(self) -> ParamStyle:
        """Get the paramstyle for this connection.

        Returns:
            ParamStyle: The paramstyle enum value
        """
        return self.__paramstyle

    @paramstyle.setter
    def paramstyle(self, value: str | ParamStyle) -> None:
        """Set binding style from a :class:`ParamStyle` or PEP 249 string (e.g. ``"pyformat"``)."""
        if isinstance(value, ParamStyle):
            self.__paramstyle = value
        elif isinstance(value, str):
            self.__paramstyle = ParamStyle.from_string(value)
        else:
            raise ProgrammingError(msg=f"paramstyle must be str or ParamStyle, got {type(value).__name__}")

    @property
    @backward_compatibility
    def _paramstyle(self) -> ParamStyle:
        """Internal binding-style storage (legacy callers assign to ``_paramstyle``)."""
        return self.__paramstyle

    @_paramstyle.setter
    @backward_compatibility
    def _paramstyle(self, value: str | ParamStyle) -> None:
        """Normalize assignments to ``_paramstyle`` (e.g. SnowPy ``temporary_paramstyle``)."""
        self.paramstyle = value

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

    @property
    @internal_api
    @backward_compatibility
    def rest(self) -> SnowflakeRestful:
        """Internal :class:`SnowflakeRestful` instance exposed for backward compatibility."""
        return SnowflakeRestful(connection=self)

    @internal_api
    def _get_connection_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        """Return connection details from Core."""
        return self.db_api.connection_get_info(
            ConnectionGetInfoRequest(
                conn_handle=self.conn_handle,
                include_master_token=include_master_token,
            )
        )

    @internal_api
    @backward_compatibility
    def _telemetry(self) -> _BackwardCompatTelemetryClient:
        return _BackwardCompatTelemetryClient()

    @property
    def role(self) -> str | None:
        """The current role in use for the session."""
        return cast("str | None", self._connection_info["role"])

    @property
    def database(self) -> str | None:
        """The current database in use for the session."""
        return cast("str | None", self._connection_info["database"])

    @property
    def schema(self) -> str | None:
        """The current schema in use for the session."""
        return cast("str | None", self._connection_info["schema"])

    @property
    def account(self) -> str | None:
        """The Snowflake account name used by this connection."""
        return cast("str | None", self._connection_info["account"])

    @property
    def warehouse(self) -> str | None:
        """The current warehouse in use for the session."""
        return cast("str | None", self._connection_info["warehouse"])

    @property
    def user(self) -> str | None:
        """The user name used for authentication."""
        return cast("str | None", self._connection_info["user"])

    @property
    def host(self) -> str | None:
        """The host name of the Snowflake instance."""
        return cast("str | None", self._connection_info["host"])

    @property
    def port(self) -> int | None:
        """The port number of the Snowflake instance."""
        return cast("int | None", self._connection_info["port"])

    @property
    def region(self) -> str | None:
        """Deprecated. The region for the Snowflake account."""
        raise NotImplementedError("region is not implemented")

    @property
    def session_id(self) -> int:
        """The Snowflake session ID for this connection."""
        value = cast("int | None", self._connection_info["session_id"])
        if value is None:
            raise InterfaceError(msg="Session ID is not available; connection may not be initialized")
        return value

    @property
    def login_timeout(self) -> int | None:
        """The login timeout in seconds."""
        raise NotImplementedError("login_timeout is not yet implemented")

    @property
    def network_timeout(self) -> int | None:
        """The network timeout in seconds for all other operations."""
        raise NotImplementedError("network_timeout is not yet implemented")

    @property
    def socket_timeout(self) -> int | None:
        """The socket timeout in seconds."""
        raise NotImplementedError("socket_timeout is not yet implemented")

    @property
    def client_session_keep_alive(self) -> bool | None:
        """Whether to keep the session active with periodic heartbeat requests."""
        raise NotImplementedError("client_session_keep_alive is not yet implemented")

    @client_session_keep_alive.setter
    def client_session_keep_alive(self, value: bool) -> None:
        raise NotImplementedError("client_session_keep_alive is not yet implemented")

    @property
    def client_session_keep_alive_heartbeat_frequency(self) -> int | None:
        """The frequency in seconds of heartbeat requests when session keep-alive is enabled."""
        raise NotImplementedError("client_session_keep_alive_heartbeat_frequency is not yet implemented")

    @client_session_keep_alive_heartbeat_frequency.setter
    def client_session_keep_alive_heartbeat_frequency(self, value: int) -> None:
        raise NotImplementedError("client_session_keep_alive_heartbeat_frequency is not yet implemented")

    @property
    def client_prefetch_threads(self) -> int:
        """The number of threads used to prefetch query result data."""
        raise NotImplementedError("client_prefetch_threads is not yet implemented")

    @client_prefetch_threads.setter
    def client_prefetch_threads(self, value: int) -> None:
        raise NotImplementedError("client_prefetch_threads is not yet implemented")

    @property
    def application(self) -> str:
        """The name of the client application connecting to Snowflake."""
        # Always set by from_connection_args (defaults to _APPLICATION_NAME)
        return self.config.application  # type: ignore[return-value]

    @property
    @pep249
    def errorhandler(self) -> Callable:
        """PEP 249 error handler called for connection and cursor errors."""
        return self._errorhandler

    @errorhandler.setter
    def errorhandler(self, value: Callable | None) -> None:
        # Bare raise: we need a working errorhandler to route errors through the protocol.
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

    @property
    def _errorhandler_connection(self) -> Connection:
        return self

    @property
    def is_pyformat(self) -> bool:
        """Whether the connection uses pyformat or format paramstyle (client-side binding)."""
        return self._paramstyle in (ParamStyle.PYFORMAT, ParamStyle.FORMAT)

    @property
    def telemetry_enabled(self) -> bool:
        """Whether client-side telemetry collection is enabled."""
        raise NotImplementedError("telemetry_enabled is not yet implemented")

    @telemetry_enabled.setter
    def telemetry_enabled(self, value: bool) -> None:
        raise NotImplementedError("telemetry_enabled is not yet implemented")

    @property
    def service_name(self) -> str | None:
        """The Snowflake service name for the connection, used for service discovery."""
        raise NotImplementedError("service_name is not yet implemented")

    @service_name.setter
    def service_name(self, value: str | None) -> None:
        raise NotImplementedError("service_name is not yet implemented")

    @property
    def log_max_query_length(self) -> int:
        """Maximum number of characters of a query string to log."""
        # ``self.config.log_max_query_length`` defaults to ``LOG_MAX_QUERY_LENGTH``
        # via the generated dataclass; the explicit fallback covers the case
        # where a caller passed ``log_max_query_length=None`` to disable the
        # default without supplying a replacement.
        return (
            self.config.log_max_query_length if self.config.log_max_query_length is not None else LOG_MAX_QUERY_LENGTH
        )

    def _format_query_for_log(self, query: str) -> str:
        """Collapse whitespace and truncate a query string for safe debug logging."""
        ret = " ".join(line.strip() for line in query.split("\n"))
        if len(ret) < self.log_max_query_length:
            return ret
        return ret[: self.log_max_query_length] + "..."

    @property
    def disable_request_pooling(self) -> bool:
        """Whether HTTP connection pooling is disabled."""
        raise NotImplementedError("disable_request_pooling is not yet implemented")

    @disable_request_pooling.setter
    def disable_request_pooling(self, value: bool) -> None:
        raise NotImplementedError("disable_request_pooling is not yet implemented")

    @property
    def use_openssl_only(self) -> bool:
        """Deprecated. Whether to restrict TLS to OpenSSL only (always ``True``)."""
        raise NotImplementedError("use_openssl_only is not yet implemented")

    @property
    def arrow_number_to_decimal(self) -> bool:
        """Whether to convert Arrow numeric types to Python ``Decimal`` instead of ``float``."""
        return bool(self.config.arrow_number_to_decimal)

    @arrow_number_to_decimal.setter
    def arrow_number_to_decimal(self, value: bool) -> None:
        self.config.arrow_number_to_decimal = bool(value)

    @arrow_number_to_decimal.setter  # type: ignore[attr-defined, untyped-decorator]
    @backward_compatibility
    def arrow_number_to_decimal_setter(self, value: bool) -> None:
        """Set arrow_number_to_decimal field. Deprecated.

        Kept so legacy code that writes
        ``cursor.connection.arrow_number_to_decimal_setter = True`` keeps
        working; new code should assign to ``arrow_number_to_decimal``
        directly.
        """
        self.arrow_number_to_decimal = value

    @property
    def validate_default_parameters(self) -> bool:
        """Whether to validate default connection parameters at connect time."""
        raise NotImplementedError("validate_default_parameters is not yet implemented")

    @property
    def insecure_mode(self) -> bool:
        """Whether OCSP certificate revocation checking is disabled."""
        raise NotImplementedError("insecure_mode is not yet implemented")

    @property
    def consent_cache_id_token(self) -> bool:
        """Whether to cache the IdP token for browser-based SSO authentication."""
        raise NotImplementedError("consent_cache_id_token is not yet implemented")

    @cached_property
    def snowflake_version(self) -> str:
        """The current Snowflake server version string."""
        with self.cursor(DictCursor) as cur:
            cur.execute("SELECT CURRENT_VERSION() AS version")
            row: dict[str, Any] = cur.fetchone()  # type: ignore[assignment]
        return str(row["VERSION"]).split(" ")[0]

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
        response = self.db_api.connection_get_query_status(
            ConnectionGetQueryStatusRequest(conn_handle=self.conn_handle, query_id=sf_qid)
        )
        try:
            status = QueryStatus[response.status_name]
        except KeyError:
            logger.warning("Unknown query status %r; treating as NO_DATA", response.status_name)
            status = QueryStatus.NO_DATA
        return status, response

    @staticmethod
    def is_still_running(status: QueryStatus) -> bool:
        """Check whether given status is currently running."""
        return status in (
            QueryStatus.RUNNING,
            QueryStatus.QUEUED,
            QueryStatus.RESUMING_WAREHOUSE,
            QueryStatus.QUEUED_REPARING_WAREHOUSE,
            QueryStatus.BLOCKED,
            QueryStatus.NO_DATA,
        )

    @staticmethod
    def is_an_error(status: QueryStatus) -> bool:
        """Check whether given status means that there has been an error."""
        return status in (
            QueryStatus.ABORTING,
            QueryStatus.FAILED_WITH_ERROR,
            QueryStatus.ABORTED,
            QueryStatus.FAILED_WITH_INCIDENT,
            QueryStatus.DISCONNECTED,
        )


# Backward compatibility alias
SnowflakeConnection = Connection
