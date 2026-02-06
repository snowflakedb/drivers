"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

from typing import Any

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (  # type: ignore[attr-defined]
    ConnectionInitRequest,
    ConnectionNewRequest,
    ConnectionSetOptionDoubleRequest,
    ConnectionSetOptionIntRequest,
    ConnectionSetOptionStringRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
)

from ._internal.api_client.client_api import database_driver_client
from .cursor import SnowflakeCursor, SnowflakeCursorBase
from .errors import InterfaceError, NotSupportedError


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
            **kwargs: Additional connection parameters
        """
        self.db_api = database_driver_client()
        self.db_handle = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle = self.db_api.connection_new(ConnectionNewRequest()).conn_handle
        for key, value in kwargs.items():
            if isinstance(value, int):
                self.db_api.connection_set_option_int(
                    ConnectionSetOptionIntRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

            if isinstance(value, str):
                self.db_api.connection_set_option_string(
                    ConnectionSetOptionStringRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

            if isinstance(value, float):
                self.db_api.connection_set_option_double(
                    ConnectionSetOptionDoubleRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

        self.db_api.connection_init(ConnectionInitRequest(conn_handle=self.conn_handle, db_handle=self.db_handle))
        self.kwargs = kwargs
        self._closed = False
        self._autocommit = False

    def close(self) -> None:
        """Close the connection now."""
        self._closed = True

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
        if self._closed:
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

        Returns:
            bool: True if connection is closed, False otherwise
        """
        return self._closed
