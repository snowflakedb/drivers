"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

import atexit
import logging
import threading
import warnings

from collections.abc import Generator, Iterable
from dataclasses import dataclass
from io import StringIO
from typing import Any, Callable, Union, cast

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionCloseRequest,
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionGetParameterRequest,
    ConnectionInitRequest,
    ConnectionIsClosedRequest,
    ConnectionNewRequest,
    ConnectionSetOptionBoolRequest,
    ConnectionSetOptionBytesRequest,
    ConnectionSetOptionDoubleRequest,
    ConnectionSetOptionIntRequest,
    ConnectionSetOptionStringRequest,
    ConnectionSetSessionParametersRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
)
from snowflake.connector._internal.snowflake_restful import SnowflakeRestful
from snowflake.connector.logout_config_mapping import (
    LogoutConfig,
    map_logout_config_phase2,
)

from ._internal._private_key_helper import normalize_private_key
from ._internal.api_client.client_api import database_driver_client
from ._internal.binding_converters import ParamStyle
from ._internal.decorators import backward_compatibility, internal_api, pep249
from ._internal.text_utils import split_statements
from .cursor import CursorInstance, CursorType, SnowflakeCursor
from .errors import InterfaceError, NotSupportedError, ProgrammingError
from .telemetry import TelemetryClient


SessionParameters = dict[str, Any]
ConnectionParamValue = Union[int, str, float, bytes, SessionParameters]
ConnectionParameters = dict[str, ConnectionParamValue]


# Module-level logger
logger = logging.getLogger(__name__)

# Error strategy constants
ERROR_STRATEGY_BEST_EFFORT = "best_effort"
ERROR_STRATEGY_STRICT = "strict"


@dataclass
class ConnectionClassConfig:
    """Static configuration flags for Connection class behavior.

    Immutable configuration that controls Connection behavior across all instances.
    """

    # Internal flag for logout semantics migration (SNOW-2314152)
    # False (default): Phase 2 - server_session_keep_alive=False respects auto-detection
    # True: Phase 3 - Pass parameters directly to Core without mapping
    # WARNING: Phase 3 will become default in future release (Breaking Change)
    USE_PHASE3_LOGOUT_SEMANTICS: bool = False


class ConnectionClassState:
    """Thread-safe process-level state shared across all Connection instances.

    This is static/class-level state, NOT instance state.
    Uses a lock to prevent race conditions in multi-threaded environments.
    """

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._first_auto_cleanup_warning_pending = True

    @property
    def first_auto_cleanup_warning_pending(self) -> bool:
        """Thread-safe getter for warning flag."""
        with self._lock:
            return self._first_auto_cleanup_warning_pending

    @first_auto_cleanup_warning_pending.setter
    def first_auto_cleanup_warning_pending(self, value: bool) -> None:
        """Thread-safe setter for warning flag."""
        with self._lock:
            self._first_auto_cleanup_warning_pending = value

    def check_and_clear_first_warning(self) -> bool:
        """Atomically check and clear the first warning flag.

        Returns:
            True if this is the first warning (and flag was cleared), False otherwise.

        This is an atomic operation that prevents race conditions where multiple
        threads might all emit the warning.
        """
        with self._lock:
            if self._first_auto_cleanup_warning_pending:
                self._first_auto_cleanup_warning_pending = False
                return True
            return False


class Connection:
    """Connection objects represent a database connection."""

    # Protected static configuration (immutable class-level settings)
    _class_config = ConnectionClassConfig()

    # Protected static state (mutable class-level state, shared across all instances)
    _class_state = ConnectionClassState()

    def __init__(self, *, paramstyle: str | None = None, **kwargs: ConnectionParamValue) -> None:
        """
        Initialize a new connection object.

        Args:
            paramstyle: Binding style – ``"pyformat"`` (default), ``"format"``, ``"qmark"`` or ``"numeric"``
            database: Database name
            user: Username
            password: Password
            host: Host name
            port: Port number
            private_key: Private key in bytes, str (base64), or RSAPrivateKey format
            session_parameters: Optional dict of session parameters to set at connection time
            server_session_keep_alive: Optional[bool] - Control server session lifecycle
                - True: Never send logout (Fire & Forget)
                - False: Respects auto-detection if enabled
                - None: Delegate to auto-detection setting
            enable_server_session_keep_alive_auto_detection: Optional[bool]
                - True: Check async query registry before logout
                - False: Don't check registry
                - None: Defaults to True (auto-detection enabled for backward compatibility)
            auto_cleanup: bool - Enable atexit handler for automatic connection cleanup
            **kwargs: Additional connection parameters
        """
        # paramstyle
        from snowflake.connector import paramstyle as default_paramstyle

        self._paramstyle = ParamStyle.from_string(paramstyle or default_paramstyle)

        kwargs = self._rewrite_private_key_password(kwargs)

        self.db_api = database_driver_client()
        self.db_handle = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle = self.db_api.connection_new(ConnectionNewRequest()).conn_handle

        # Extract session_parameters before processing other kwargs
        session_params: SessionParameters | None = kwargs.pop("session_parameters", None)  # type: ignore

        # Pre-process private_key if present - normalize for Rust core
        if "private_key" in kwargs:
            kwargs["private_key"] = normalize_private_key(kwargs["private_key"])

        # Extract logout configuration parameters before passing to Core
        self.server_session_keep_alive: bool | None = cast("bool | None", kwargs.pop("server_session_keep_alive", None))
        self.enable_server_session_keep_alive_auto_detection: bool | None = cast(
            "bool | None", kwargs.pop("enable_server_session_keep_alive_auto_detection", None)
        )
        self.auto_cleanup: bool = cast(bool, kwargs.pop("auto_cleanup", True))

        for key, value in kwargs.items():
            if isinstance(value, int):
                self.db_api.connection_set_option_int(
                    ConnectionSetOptionIntRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

            elif isinstance(value, str):
                self.db_api.connection_set_option_string(
                    ConnectionSetOptionStringRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

            elif isinstance(value, float):
                self.db_api.connection_set_option_double(
                    ConnectionSetOptionDoubleRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

            elif isinstance(value, bytes):
                self.db_api.connection_set_option_bytes(
                    ConnectionSetOptionBytesRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

        # Set session parameters if provided (before connection_init)
        if session_params:
            self.db_api.connection_set_session_parameters(
                ConnectionSetSessionParametersRequest(conn_handle=self.conn_handle, parameters=session_params)
            )

        # Configure logout behavior BEFORE connection_init (init-time configuration)
        # Map logout parameters using Phase 2 semantics for backward compatibility
        logout_config = self._map_logout_config()

        # Set logout configuration via ConnectionSetOption* calls
        if logout_config.server_session_keep_alive is not None:
            self.db_api.connection_set_option_bool(
                ConnectionSetOptionBoolRequest(
                    conn_handle=self.conn_handle,
                    key="server_session_keep_alive",
                    value=logout_config.server_session_keep_alive,
                )
            )

        if logout_config.enable_auto_detection is not None:
            self.db_api.connection_set_option_bool(
                ConnectionSetOptionBoolRequest(
                    conn_handle=self.conn_handle,
                    key="enable_logout_auto_detection",
                    value=logout_config.enable_auto_detection,
                )
            )

        # Set error strategy (always set, has default)
        error_strategy_str = ERROR_STRATEGY_BEST_EFFORT if logout_config.error_strategy == 1 else ERROR_STRATEGY_STRICT
        self.db_api.connection_set_option_string(
            ConnectionSetOptionStringRequest(
                conn_handle=self.conn_handle,
                key="logout_error_strategy",
                value=error_strategy_str,
            )
        )

        # Set timeout and retry configuration
        self.db_api.connection_set_option_int(
            ConnectionSetOptionIntRequest(
                conn_handle=self.conn_handle,
                key="logout_total_timeout_seconds",
                value=logout_config.logout_total_timeout_seconds,
            )
        )

        if logout_config.max_retry_attempts is not None:
            self.db_api.connection_set_option_int(
                ConnectionSetOptionIntRequest(
                    conn_handle=self.conn_handle,
                    key="logout_max_retry_attempts",
                    value=logout_config.max_retry_attempts,
                )
            )

        if logout_config.logout_request_timeout_seconds is not None:
            self.db_api.connection_set_option_int(
                ConnectionSetOptionIntRequest(
                    conn_handle=self.conn_handle,
                    key="logout_request_timeout_seconds",
                    value=logout_config.logout_request_timeout_seconds,
                )
            )

        self.db_api.connection_init(ConnectionInitRequest(conn_handle=self.conn_handle, db_handle=self.db_handle))
        _sensitive_keys = {"password", "private_key"}
        self.kwargs = {k: ("***" if k in _sensitive_keys else v) for k, v in kwargs.items()}
        self._closed = False
        self._autocommit = False
        self._messages: list[tuple[type[Exception], dict[str, str | bool]]] = []
        self._errorhandler: Callable

        # Register atexit handler if auto_cleanup is enabled
        if self.auto_cleanup:
            atexit.register(self._close_at_process_exit)

    def _map_logout_config(self) -> LogoutConfig:
        """Map logout parameters to Core configuration.

        Returns logout configuration with all values resolved (defaults applied,
        phase-specific mapping done).

        Related: SNOW-2314152
        """
        return map_logout_config_phase2(self)

    @pep249
    def close(self, retry: bool = True) -> None:
        """
        Close the connection now.

        Sends logout request to server based on configuration set at connection initialization,
        with optional per-request overrides.

        Args:
            retry: If False, disables logout retries for this close operation only.
                   Overrides connection-wide max_retry_attempts to 0.
                   If True, uses the connection-wide configured value.

        Behavior (Phase 2 - Backward Compatible, SNOW-2314152):
            - Auto-detection enabled by default (legacy Python behavior for backward compatibility)
            - server_session_keep_alive=False still respects auto-detection
            - server_session_keep_alive=True never sends logout (Fire & Forget)
            - server_session_keep_alive=None delegates to auto-detection setting
            - Logout configuration is set at connection initialization time (init-time)
            - retry parameter provides close-time override for max_retry_attempts only

        Configuration Hierarchy:
            close-time override (retry parameter) > connection-wide (init-time) > defaults

        Note: This matches the architecture of all existing Snowflake drivers
        (Go, JDBC, .NET, Node.js) while supporting Python's legacy retry parameter.
        """
        # Unregister atexit handler to prevent it from running at process exit
        # after explicit close(). This prevents double cleanup and false warnings.
        # atexit.unregister() is idempotent, safe to call multiple times.
        atexit.unregister(self._close_at_process_exit)

        # Note: Idempotence is handled atomically in Core (connection_close)

        # Dual config approach: override max_retry_attempts at close-time if retry=False
        # - retry=True: Pass None → Rust hierarchy uses connection-wide value
        # - retry=False: Pass 0 → Rust hierarchy overrides (0 retries = disabled, 1 attempt only)

        max_retry_override = None if retry else 0

        # Call Core connection_close with optional override
        # When None is passed, Rust's merge_with_request uses connection-wide configured value
        self.db_api.connection_close(
            ConnectionCloseRequest(
                conn_handle=self.conn_handle,
                max_retry_attempts=max_retry_override,
            )
        )

    def _close_at_process_exit(self) -> None:
        """
        Cleanup handler called by atexit when process exits.

        If close() was called successfully, this handler should have been unregistered
        and should NOT run. If it runs for an already-closed connection, that indicates
        a potential bug (unregister failed, race condition, or multiple registrations).
        """
        if self.is_closed():
            # This shouldn't happen! If close() succeeded, handler should be unregistered.
            logger.debug(
                "atexit handler ran for already-closed connection. "
                "This may indicate atexit.unregister() failed or a race condition occurred."
            )
            return

        # Connection is leaked (not explicitly closed) - emit deprecation warning
        # Phase 3 (SNOW-2314152): Auto-cleanup will be disabled by default
        # Atomically check and clear warning flag (thread-safe)
        if self.__class__._class_state.check_and_clear_first_warning():
            warnings.warn(
                "Connection was not explicitly closed before process exit. "
                "Auto-cleanup at exit will be disabled by default in a future version. "
                "Please explicitly call connection.close() or use context manager.",
                FutureWarning,
                stacklevel=2,
            )

        # Attempt cleanup for leaked connection
        try:
            # Temporarily disable auto_cleanup flag to avoid atexit recursion
            saved_auto_cleanup = self.auto_cleanup
            self.auto_cleanup = False
            self.close(retry=False)
            self.auto_cleanup = saved_auto_cleanup
        except Exception as e:
            logger.warning(f"Failed to cleanup connection at exit: {e}")
            # Suppress error - can't propagate from atexit handler

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], dict[str, str | bool]]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], dict[str, str | bool]]]) -> None:
        self._messages = value

    @pep249
    def commit(self) -> None:
        """
        Commit any pending transaction to the database.

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("commit is not implemented")

    @pep249
    def rollback(self) -> None:
        """
        Roll back to the start of any pending transaction.

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("rollback is not implemented")

    @pep249
    def cursor(self, cursor_class: CursorType = SnowflakeCursor) -> CursorInstance:
        """
        Return a new Cursor object using the connection.

        Args:
            cursor_class: The class to use for the cursor (default: SnowflakeCursor).
                          Pass DictCursor to get results as dictionaries.

        Returns:
            SnowflakeCursorBase: A new cursor object
        """
        if self.is_closed():
            raise InterfaceError("Connection is closed")
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
        """
        Exit the runtime context for the connection.

        If an exception occurred, rollback the transaction.
        Otherwise, commit the transaction.
        """
        if exc_type is None:
            # No exception, commit
            try:
                self.commit()
            except NotSupportedError:
                pass  # commit not implemented
        else:
            # Exception occurred, rollback
            try:
                self.rollback()
            except NotSupportedError:
                pass  # rollback not implemented

        self.close()

    # Optional methods that some databases might support
    def cancel(self) -> None:
        """
        Cancel a long-running operation on the connection.

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("cancel is not implemented")

    def ping(self) -> bool:
        """
        Check if the connection to the server is still alive.

        Returns:
            bool: True if connection is alive, False otherwise

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("ping is not implemented")

    def set_autocommit(self, autocommit: bool) -> None:
        """
        Set the autocommit mode.

        Args:
            autocommit (bool): True to enable autocommit, False to disable
        """
        # TODO: SNOW-3155976 Lacks full implementation
        self._autocommit = autocommit

    def get_autocommit(self) -> bool:
        """
        Get the current autocommit mode.

        Returns:
            bool: Current autocommit setting
        """
        # TODO: SNOW-3155976 Lacks full implementation
        return self._autocommit

    @pep249
    def autocommit(self, value: bool) -> None:
        """
        Set autocommit mode.

        Args:
            value (bool): Autocommit setting
        """
        self._autocommit = value
        self.set_autocommit(value)

    def is_closed(self) -> bool:
        """
        Check if the connection is closed.

        Queries the Core's authoritative closed state rather than maintaining
        a separate Python-side flag.

        Returns:
            bool: True if close() has been called (connection marked as closed atomically),
                  False if close() has never been called

        Important: Core sets is_closed=True immediately when close() starts, BEFORE
        attempting logout. This means is_closed() returns True even if:
        - Logout fails and close() raises an exception (error_strategy=STRICT)
        - Logout is still in progress
        This ensures idempotency and prevents double-close attempts.
        """
        response = self.db_api.connection_is_closed(ConnectionIsClosedRequest(conn_handle=self.conn_handle))
        return bool(response.is_closed)

    def _get_session_parameter(self, name: str) -> str | None:
        """
        Get a session parameter value (internal method).

        Args:
            name: The parameter name (case-insensitive)

        Returns:
            str | None: The parameter value, or None if not found
        """
        request = ConnectionGetParameterRequest(conn_handle=self.conn_handle, key=name)
        response = self.db_api.connection_get_parameter(request)
        return response.value if response.value else None

    @property
    def paramstyle(self) -> ParamStyle:
        """Get the paramstyle for this connection.

        Returns:
            ParamStyle: The paramstyle enum value
        """
        return self._paramstyle

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
    def _get_connection_info(self) -> ConnectionGetInfoResponse:
        """Refresh connection details for connection"""
        return self.db_api.connection_get_info(ConnectionGetInfoRequest(conn_handle=self.conn_handle))

    @internal_api
    @backward_compatibility
    def _telemetry(self) -> TelemetryClient:
        return TelemetryClient()

    @backward_compatibility
    def _rewrite_private_key_password(self, kwargs: ConnectionParameters) -> ConnectionParameters:
        private_key_file_pwd = kwargs.pop("private_key_file_pwd", None)
        if private_key_file_pwd is not None:
            kwargs = {**kwargs, "private_key_password": private_key_file_pwd}
        return kwargs

    @property
    def role(self) -> str | None:
        """The current role in use for the session."""
        return self.kwargs.get("role")  # type: ignore[return-value]

    @property
    def database(self) -> str | None:
        """The current database in use for the session."""
        # TODO: SNOW-3155976 Read from connection details
        return self.kwargs.get("database")  # type: ignore[return-value]

    @property
    def schema(self) -> str | None:
        """The current schema in use for the session."""
        # TODO: SNOW-3155976 Read from connection details
        return self.kwargs.get("schema")  # type: ignore[return-value]

    @property
    def account(self) -> str | None:
        """The Snowflake account name used by this connection."""
        # TODO: SNOW-3155976 Read from connection details
        return self.kwargs.get("account")  # type: ignore[return-value]

    @property
    def warehouse(self) -> str | None:
        """The current warehouse in use for the session."""
        # TODO: SNOW-3155976 Read from connection details
        return self.kwargs.get("warehouse")  # type: ignore[return-value]

    @property
    def user(self) -> str | None:
        """The user name used for authentication."""
        # TODO: SNOW-3155976 Read from connection details
        return self.kwargs.get("user")  # type: ignore[return-value]

    @property
    def host(self) -> str | None:
        """The host name of the Snowflake instance."""
        # TODO: SNOW-3155976 Read from connection details
        return self.kwargs.get("host")  # type: ignore[return-value]

    @property
    def port(self) -> int | None:
        """The port number of the Snowflake instance."""
        # TODO: SNOW-3155976 Read from connection details
        return self.kwargs.get("port")  # type: ignore[return-value]

    @property
    def region(self) -> str | None:
        """Deprecated. The region for the Snowflake account."""
        raise NotImplementedError("region is not implemented")

    @property
    def session_id(self) -> int:
        """The Snowflake session ID for this connection."""
        # TODO: SNOW-3155976 Read from connection details
        raise NotImplementedError("session_id is not yet implemented")

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
        raise NotImplementedError("application is not yet implemented")

    @property
    @pep249
    def errorhandler(self) -> Callable:
        """PEP 249 error handler called for connection and cursor errors."""
        return self._errorhandler

    @errorhandler.setter
    def errorhandler(self, value: Callable | None) -> None:
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

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
        raise NotImplementedError("log_max_query_length is not yet implemented")

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
        raise NotImplementedError("arrow_number_to_decimal is not yet implemented")

    @arrow_number_to_decimal.setter
    def arrow_number_to_decimal(self, value: bool) -> None:
        raise NotImplementedError("arrow_number_to_decimal is not yet implemented")

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

    @property
    def snowflake_version(self) -> str:
        """The current Snowflake server version string."""
        raise NotImplementedError("snowflake_version is not yet implemented")

    def get_query_status(self, sf_qid: str) -> Any:
        """Retrieve the status of query with sf_qid."""
        raise NotImplementedError("get_query_status is not yet implemented")

    def get_query_status_throw_if_error(self, sf_qid: str) -> Any:
        """Retrieve the status of query with sf_qid and raises an exception if the query terminated with an error."""
        raise NotImplementedError("get_query_status_throw_if_error is not yet implemented")

    @staticmethod
    def is_still_running(status: Any) -> bool:
        """Check whether given status is currently running."""
        raise NotImplementedError("is_still_running is not yet implemented")

    @staticmethod
    def is_an_error(status: Any) -> bool:
        """Check whether given status means that there has been an error."""
        raise NotImplementedError("is_an_error is not yet implemented")


# Backward compatibility alias
SnowflakeConnection = Connection
