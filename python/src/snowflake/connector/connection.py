"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

import atexit
import logging
import warnings

from collections.abc import Generator, Iterable
from dataclasses import dataclass
from io import StringIO
from typing import Any, Callable, Union

from snowflake.connector._internal.protobuf_gen import database_driver_v1_pb2
from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionCloseRequest,
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionGetParameterRequest,
    ConnectionInitRequest,
    ConnectionIsClosedRequest,
    ConnectionNewRequest,
    ConnectionSetOptionBytesRequest,
    ConnectionSetOptionDoubleRequest,
    ConnectionSetOptionIntRequest,
    ConnectionSetOptionStringRequest,
    ConnectionSetSessionParametersRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
)
from snowflake.connector._internal.snowflake_restful import SnowflakeRestful
from snowflake.connector.logout_params_mapping_strategy import (
    LogoutParamsMappingStrategy,
    Phase2LogoutParamsMappingStrategy,
    Phase3LogoutParamsMappingStrategy,
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


@dataclass
class ConnectionClassState:
    """Mutable class-level state shared across all Connection instances in the process.

    This is static/class-level state, NOT instance state.
    """

    # Track if first auto-cleanup warning has been emitted in this process
    # False = warning already emitted, True = warning not yet emitted
    first_auto_cleanup_warning_pending: bool = True


class Connection:
    """Connection objects represent a database connection."""

    # Protected static configuration (immutable class-level settings)
    _class_config = ConnectionClassConfig()

    # Protected static state (mutable class-level state, shared across all instances)
    _class_state = ConnectionClassState()

    @classmethod
    def _get_logout_params_mapping_strategy(cls) -> LogoutParamsMappingStrategy:
        """Create a logout parameter mapping strategy.

        Returns:
            Phase2 or Phase3 strategy based on USE_PHASE3_LOGOUT_SEMANTICS flag.
        """
        if cls._class_config.USE_PHASE3_LOGOUT_SEMANTICS:
            return Phase3LogoutParamsMappingStrategy()
        return Phase2LogoutParamsMappingStrategy()

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
        self.server_session_keep_alive: bool | None = kwargs.pop("server_session_keep_alive", None)
        self.enable_server_session_keep_alive_auto_detection: bool | None = kwargs.pop(
            "enable_server_session_keep_alive_auto_detection", None
        )
        self.auto_cleanup: bool = kwargs.pop("auto_cleanup", True)

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

    @pep249
    def close(self, retry: bool = True) -> None:
        """
        Close the connection now.

        Sends logout request to server based on configuration, then cleans up resources.

        Args:
            retry: Whether to retry failed logout requests (backward compatible with old driver).
                   - True (default): Allow retries (Core default: up to 6 attempts with exponential backoff)
                   - False: No retries, single attempt only (matches old driver behavior)

        Behavior (Phase 2 - Backward Compatible, SNOW-2314152):
            - Auto-detection enabled by default (legacy Python behavior for backward compatibility)
            - server_session_keep_alive=False still respects auto-detection
            - server_session_keep_alive=True never sends logout (Fire & Forget)
            - server_session_keep_alive=None delegates to auto-detection setting

        Note: Phase 2 behavior is achieved by mapping Python parameters to Core's Phase 3
        semantics. In Phase 3, Python will pass parameters directly to Core without mapping.
        See .ai/docs/UD_Design_Doc_Fire_Forget.md and SNOW-2314152 for Phase 2/3 migration plan.
        """
        # Unregister atexit handler to prevent it from running at process exit
        # after explicit close(). This prevents double cleanup and false warnings.
        # atexit.unregister() is idempotent, safe to call multiple times.
        atexit.unregister(self._close_at_process_exit)

        # Note: Idempotence is handled atomically in Core (connection_close)

        # Map Python parameters to Core parameters using strategy pattern (SNOW-2314152)
        strategy = self._get_logout_params_mapping_strategy()
        core_keep_alive = strategy.map_server_session_keep_alive(self)

        # Default auto-detection setting for Core (used when core_keep_alive is None)
        effective_enable_auto = (
            self.enable_server_session_keep_alive_auto_detection
            if self.enable_server_session_keep_alive_auto_detection is not None
            else True
        )

        # Handle retry parameter (backward compatibility with old driver)
        # Old driver: retry=True → 3 attempts, retry=False → 1 attempt
        # UD: Pass max_retry_attempts to Core to control retry count
        if retry:
            # Allow retries: Use Core default (typically 6 attempts)
            max_retry_attempts = None
        else:
            # No retries: Single attempt only (matches old driver retry=False)
            max_retry_attempts = 1

        # Call Core connection_close with mapped configuration
        # Core will set is_closed flag atomically
        self.db_api.connection_close(
            ConnectionCloseRequest(
                conn_handle=self.conn_handle,
                server_session_keep_alive=core_keep_alive,  # Mapped parameter
                enable_auto_detection=effective_enable_auto,
                error_strategy=database_driver_v1_pb2.ERROR_STRATEGY_BEST_EFFORT,  # Python default
                timeout_seconds=5,  # 5 second default
                max_retry_attempts=max_retry_attempts,
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
        if self.__class__._class_state.first_auto_cleanup_warning_pending:
            warnings.warn(
                "Connection was not explicitly closed before process exit. "
                "Auto-cleanup at exit will be disabled by default in a future version. "
                "Please explicitly call connection.close() or use context manager.",
                FutureWarning,
                stacklevel=2,
            )
            self.__class__._class_state.first_auto_cleanup_warning_pending = False

        # Attempt cleanup for leaked connection
        try:
            # Temporarily disable auto_cleanup flag to avoid atexit recursion
            saved_auto_cleanup = self.auto_cleanup
            self.auto_cleanup = False
            self.close(retry=False)
            self.auto_cleanup = saved_auto_cleanup
        except Exception:
            pass  # Suppress errors during exit cleanup

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
            bool: True if connection is closed, False otherwise
        """
        try:
            response = self.db_api.connection_is_closed(ConnectionIsClosedRequest(conn_handle=self.conn_handle))
            return bool(response.is_closed)
        except InterfaceError:
            # If handle is invalid or already released, treat as closed
            # This can happen if connection_release() was called
            return True

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
