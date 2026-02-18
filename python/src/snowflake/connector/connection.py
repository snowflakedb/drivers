"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

import atexit
import logging
import warnings

from typing import Any

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionCloseRequest,
    ConnectionInitRequest,
    ConnectionIsClosedRequest,
    ConnectionNewRequest,
    ConnectionSetOptionBytesRequest,
    ConnectionSetOptionDoubleRequest,
    ConnectionSetOptionIntRequest,
    ConnectionSetOptionStringRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
)

from ._internal._private_key_helper import normalize_private_key
from ._internal.api_client.client_api import database_driver_client
from .cursor import SnowflakeCursor, SnowflakeCursorBase
from .errors import InterfaceError, NotSupportedError


# Module-level logger
logger = logging.getLogger(__name__)

# Global flag to track if first auto-cleanup warning has been emitted in this process
_first_auto_cleanup_in_process = True


class Connection:
    """Connection objects represent a database connection."""

    def __init__(self, **kwargs: Any) -> None:
        """
        Initialize a new connection object.

        Args:
            database: Database name
            user: Username
            password: Password
            host: Host name
            port: Port number
            private_key: Private key in bytes, str (base64), or RSAPrivateKey format
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
        self.db_api = database_driver_client()
        self.db_handle = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle = self.db_api.connection_new(ConnectionNewRequest()).conn_handle

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

        self.db_api.connection_init(ConnectionInitRequest(conn_handle=self.conn_handle, db_handle=self.db_handle))
        self.kwargs = kwargs
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
            retry: Whether to retry failed logout (Note: retry parameter kept for compatibility,
                   but retry behavior is now controlled by Core's retry policy)

        Behavior:
            - Auto-detection enabled by default (legacy Python behavior for backward compatibility)
            - server_session_keep_alive=False still respects auto-detection
            - server_session_keep_alive=True never sends logout (Fire & Forget)
            - server_session_keep_alive=None delegates to auto-detection setting
        """
        # Unregister atexit handler to prevent it from running at process exit
        # after explicit close(). This prevents double cleanup and false warnings.
        # atexit.unregister() is idempotent, safe to call multiple times.
        atexit.unregister(self._close_at_process_exit)

        if self.is_closed():
            return  # Already closed, idempotent

        # Default to True (auto-detection enabled for backward compatibility)
        effective_enable_auto = (
            self.enable_server_session_keep_alive_auto_detection
            if self.enable_server_session_keep_alive_auto_detection is not None
            else True
        )

        # Call Core connection_close with configuration
        # Core will set is_closed flag atomically
        self.db_api.connection_close(
            ConnectionCloseRequest(
                conn_handle=self.conn_handle,
                server_session_keep_alive=self.server_session_keep_alive,
                enable_auto_detection=effective_enable_auto,
                error_strategy="BestEffort",  # Python default
                timeout_seconds=5,  # 5 second default
            )
        )

    def _close_at_process_exit(self) -> None:
        """
        Cleanup handler called by atexit when process exits.

        If close() was called successfully, this handler should have been unregistered
        and should NOT run. If it runs for an already-closed connection, that indicates
        a potential bug (unregister failed, race condition, or multiple registrations).
        """
        global _first_auto_cleanup_in_process

        if self.is_closed():
            # This shouldn't happen! If close() succeeded, handler should be unregistered.
            logger.debug(
                "atexit handler ran for already-closed connection. "
                "This may indicate atexit.unregister() failed or a race condition occurred."
            )
            return

        # Connection is leaked (not explicitly closed) - emit deprecation warning
        if _first_auto_cleanup_in_process:
            warnings.warn(
                "Connection was not explicitly closed before process exit. "
                "Auto-cleanup at exit will be disabled by default in Phase 3. "
                "Please explicitly call connection.close() or use context manager.",
                FutureWarning,
                stacklevel=2,
            )
            _first_auto_cleanup_in_process = False

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
