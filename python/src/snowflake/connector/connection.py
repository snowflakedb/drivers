"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

from collections.abc import Generator, Iterable
from io import StringIO
from typing import Any, Union

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (  # type: ignore[attr-defined]
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionInitRequest,
    ConnectionNewRequest,
    ConnectionSetOptionBytesRequest,
    ConnectionSetOptionDoubleRequest,
    ConnectionSetOptionIntRequest,
    ConnectionSetOptionStringRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
)
from snowflake.connector._internal.snowflake_restful import SnowflakeRestful

from ._internal import internal_api
from ._internal._private_key_helper import normalize_private_key
from ._internal.api_client.client_api import database_driver_client
from ._internal.decorators import backward_compatibility
from ._internal.text_utils import split_statements
from .cursor import Cursor
from .errors import InterfaceError, NotSupportedError


ConnectionParamValue = Union[int, str, float]
ConnectionParameters = dict[str, ConnectionParamValue]


class Connection:
    """Connection objects represent a database connection."""

    def __init__(self, **kwargs: ConnectionParamValue) -> None:
        """
        Initialize a new connection object.

        Args:
            database: Database name
            user: Username
            password: Password
            host: Host name
            port: Port number
            private_key: Private key in bytes, str (base64), or RSAPrivateKey format
            **kwargs: Additional connection parameters
        """
        kwargs = self._check_if_read_from_config(kwargs)
        kwargs = self._rewrite_private_key_password(kwargs)

        self.db_api = database_driver_client()
        self.db_handle = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle = self.db_api.connection_new(ConnectionNewRequest()).conn_handle

        # Pre-process private_key if present - normalize for Rust core
        if "private_key" in kwargs:
            kwargs["private_key"] = normalize_private_key(kwargs["private_key"])  # type: ignore

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

            elif isinstance(value, bytes):  # type: ignore
                self.db_api.connection_set_option_bytes(
                    ConnectionSetOptionBytesRequest(conn_handle=self.conn_handle, key=key, value=value)
                )

        self.db_api.connection_init(ConnectionInitRequest(conn_handle=self.conn_handle, db_handle=self.db_handle))
        self._connection_info: ConnectionGetInfoResponse = self.db_api.connection_get_info(
            ConnectionGetInfoRequest(conn_handle=self.conn_handle)
        )
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

    def cursor(self, cursor_class: type[Cursor] = Cursor) -> Cursor:
        """
        Return a new Cursor object using the connection.

        Returns:
            Cursor: A new cursor object
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

    @backward_compatibility
    def execute_string(
        self,
        sql_text: str,
        remove_comments: bool = False,
        return_cursors: bool = True,
        cursor_class: type[Cursor] = Cursor,
        **kwargs: dict,
    ) -> Iterable[Cursor]:
        """Execute a SQL text including multiple statements. This is a non-standard convenience method."""
        stream = StringIO(sql_text)
        stream_generator = self.execute_stream(
            stream, remove_comments=remove_comments, cursor_class=cursor_class, **kwargs
        )
        ret = list(stream_generator)
        return ret if return_cursors else list()

    @backward_compatibility
    def execute_stream(
        self,
        stream: StringIO,
        remove_comments: bool = False,
        cursor_class: type[Cursor] = Cursor,
        **kwargs: dict,
    ) -> Generator[Cursor]:
        """Execute a stream of SQL statements. This is a non-standard convenient method."""
        split_statements_list = split_statements(stream, remove_comments=remove_comments)
        # Note: split_statements_list is a list of tuples of sql statements and whether they are put/get
        non_empty_statements = [e for e in split_statements_list if e[0]]
        for sql, is_put_or_get in non_empty_statements:
            cur = self.cursor(cursor_class=cursor_class)
            cur.execute(sql, _is_put_get=is_put_or_get, **kwargs)
            yield cur

    @property
    @internal_api
    @backward_compatibility
    def rest(self) -> SnowflakeRestful:
        return SnowflakeRestful(connection_info=self._connection_info)

    @internal_api
    @backward_compatibility
    def _telemetry(self) -> Any:
        return None

    @backward_compatibility
    def _check_if_read_from_config(self, kwargs: ConnectionParameters) -> ConnectionParameters:
        if "connection_name" in kwargs:
            from snowflake.connector.config_manager import CONFIG_MANAGER

            connection_details = dict(CONFIG_MANAGER["connections"][kwargs["connection_name"]])
            return connection_details
        return kwargs

    @backward_compatibility
    def _rewrite_private_key_password(self, kwargs: ConnectionParameters) -> ConnectionParameters:
        if "private_key_file_pwd" in kwargs:
            kwargs = {**kwargs, "private_key_password": kwargs["private_key_file_pwd"]}
        return kwargs

    @property
    def role(self) -> str | None:
        return self.kwargs.get("role")  # type: ignore[return-value]

    @property
    def database(self) -> str | None:
        return self.kwargs.get("database")  # type: ignore[return-value]

    @property
    def schema(self) -> str | None:
        return self.kwargs.get("schema")  # type: ignore[return-value]


# Backward compatibility alias
SnowflakeConnection = Connection
