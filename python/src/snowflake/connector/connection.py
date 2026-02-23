"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

import atexit
import logging
import warnings

from dataclasses import dataclass
from typing import Any

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionCloseRequest,
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
from snowflake.connector._internal.protobuf_gen import database_driver_v1_pb2

from ._internal._private_key_helper import normalize_private_key
from ._internal.api_client.client_api import database_driver_client
from .cursor import SnowflakeCursor, SnowflakeCursorBase
from .errors import InterfaceError, NotSupportedError, ProgrammingError


# Paramstyles that enable server-side binding in the universal driver.
_SUPPORTED_PARAMSTYLES = {"qmark", "numeric"}

# TODO: to be added in follow-up PR
_CLIENT_SIDE_PARAMSTYLES = {"format", "pyformat"}


def _resolve_paramstyle(value: str | None) -> str | None:
    """Validate a *paramstyle* value.

    Returns the canonical lower-case paramstyle string when it names a
    server-side binding style supported by the universal driver, ``None``
    when it names a client-side style that we tolerate but don't support,
    and raises :class:`ProgrammingError` for anything else.
    """
    if value is None:
        return None

    normalised = value.strip().lower()

    if normalised in _SUPPORTED_PARAMSTYLES:
        return normalised

    # TODO: remove in follow-up PR
    if normalised in _CLIENT_SIDE_PARAMSTYLES:
        return None

    raise ProgrammingError(
        f"Invalid paramstyle is specified: {value!r}. Supported values: {', '.join(sorted(_SUPPORTED_PARAMSTYLES))}"
    )


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

    # Private static configuration (immutable class-level settings, name-mangled)
    __class_config = ConnectionClassConfig()

    # Private static state (mutable class-level state, shared across all instances, name-mangled)
    __class_state = ConnectionClassState()

    def __init__(self, *, paramstyle: str | None = None, **kwargs: Any) -> None:
        """
        Initialize a new connection object.

        Args:
            paramstyle: Binding style – ``"qmark"`` or ``"numeric"``.
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
        self._paramstyle = _resolve_paramstyle(paramstyle)

        self.db_api = database_driver_client()
        self.db_handle = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle = self.db_api.connection_new(ConnectionNewRequest()).conn_handle

        # Extract session_parameters before processing other kwargs
        session_params = kwargs.pop("session_parameters", None)

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
        self._closed = False
        self._autocommit = False

        # Register atexit handler if auto_cleanup is enabled
        if self.auto_cleanup:
            atexit.register(self._close_at_process_exit)

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

        # Default to True (auto-detection enabled for backward compatibility)
        effective_enable_auto = (
            self.enable_server_session_keep_alive_auto_detection
            if self.enable_server_session_keep_alive_auto_detection is not None
            else True
        )

        # Phase 2/3 Parameter Mapping (SNOW-2314152)
        if Connection.__class_config.USE_PHASE3_LOGOUT_SEMANTICS:
            # Phase 3: Pass parameters directly to Core without mapping
            core_keep_alive = self.server_session_keep_alive
        else:
            # Phase 2 (default): Map Python Phase 2 semantics to Core's Phase 3 semantics
            # Python Phase 2: server_session_keep_alive=False respects auto-detection when enabled
            # Core Phase 3: Some(false) = always logout, ignores auto-detection
            #
            # Mapping:
            # - Python: False + auto-detection enabled → Core: None (respects auto-detection)
            # - Python: False + auto-detection disabled → Core: False (force logout)
            # - Python: True → Core: True (never logout)
            # - Python: None → Core: None (delegate to auto-detection)
            core_keep_alive = self.server_session_keep_alive
            if self.server_session_keep_alive is False and effective_enable_auto:
                # Phase 2 compat: False with auto-detection enabled → map to None
                # This makes Core check the registry (legacy Python behavior)
                core_keep_alive = None

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
        if Connection.__class_state.first_auto_cleanup_warning_pending:
            warnings.warn(
                "Connection was not explicitly closed before process exit. "
                "Auto-cleanup at exit will be disabled by default in Phase 3 (SNOW-2314152). "
                "Please explicitly call connection.close() or use context manager.",
                FutureWarning,
                stacklevel=2,
            )
            Connection.__class_state.first_auto_cleanup_warning_pending = False

        # Attempt cleanup for leaked connection
        try:
            # Temporarily disable auto_cleanup flag to avoid atexit recursion
            saved_auto_cleanup = self.auto_cleanup
            self.auto_cleanup = False
            self.close(retry=False)
            self.auto_cleanup = saved_auto_cleanup
        except Exception:
            pass  # Suppress errors during exit cleanup

    def commit(self) -> None:
        """
        Commit any pending transaction to the database.

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("commit is not implemented")

    def rollback(self) -> None:
        """
        Roll back to the start of any pending transaction.

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("rollback is not implemented")

    def cursor(self, cursor_class: type[SnowflakeCursorBase] = SnowflakeCursor) -> SnowflakeCursorBase:
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

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("set_autocommit is not implemented")

    def get_autocommit(self) -> bool:
        """
        Get the current autocommit mode.

        Returns:
            bool: Current autocommit setting

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("get_autocommit is not implemented")

    @property
    def autocommit(self) -> bool:
        """
        Get/set autocommit mode as a property.

        Returns:
            bool: Current autocommit setting
        """
        return self._autocommit

    @autocommit.setter
    def autocommit(self, value: bool) -> None:
        """
        Set autocommit mode.

        Args:
            value (bool): Autocommit setting
        """
        self._autocommit = value
        try:
            self.set_autocommit(value)
        except NotSupportedError:
            pass  # autocommit not supported by implementation

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
