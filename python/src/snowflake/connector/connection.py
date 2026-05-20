"""PEP 249 Database API 2.0 Connection Objects.

:class:`Connection` is a thin synchronous wrapper over
:class:`~snowflake.connector.aio.connection.AsyncConnection`.
"""

from __future__ import annotations

import asyncio
import atexit
import functools
import logging
import threading
import warnings

from collections.abc import Callable, Generator, Iterable
from functools import cached_property
from io import StringIO
from typing import Any, TypeVar, cast

from ._internal.api_client.client_api import connection_close_at_exit, get_background_loop
from ._internal.binding_converters import ParamStyle
from ._internal.decorators import api_telemetry, backward_compatibility, internal_api, pep249
from ._internal.errorcode import ER_CONNECTION_IS_CLOSED, ER_INVALID_VALUE
from ._internal.errorhandler import ErrorHandlerMixin
from ._internal.extras import check_dependency  # noqa: F401 — patch target for tests
from ._internal.extras import numpy as np  # noqa: F401 — patch target for tests
from ._internal.protobuf_gen.database_driver_v1_services import ConnectionGetInfoResponse
from ._internal.snowflake_restful import SnowflakeRestful
from ._internal.sqlstate import SQLSTATE_CONNECTION_NOT_EXISTS
from .aio.connection import AsyncConnection
from .connection_config import ConnectionConfig
from .constants import QueryStatus
from .cursor import CursorInstance, CursorType, DictCursor, SnowflakeCursor
from .errors import DatabaseError, Error, ErrorValue, ProgrammingError


# snowflake-sqlalchemy imports this symbol
DEFAULT_CONFIGURATION: dict[str, tuple[Any, tuple[type, ...]]] = {}

_APPLICATION_NAME = "PythonConnector"
CLIENT_NAME = _APPLICATION_NAME

LOG_MAX_QUERY_LENGTH = 80

SessionParameters = dict[str, Any]
ConnectionParamValue = int | str | float | bytes | bool | SessionParameters
ConnectionParameters = dict[str, ConnectionParamValue]

logger = logging.getLogger(__name__)

F = TypeVar("F", bound=Callable[..., Any])


def _requires_open(func: F) -> F:
    """Raise ``DatabaseError`` if the connection is closed."""

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
    """Synchronous connection — a blocking wrapper over :class:`AsyncConnection`."""

    def __init__(
        self,
        *,
        connection_name: str | None = None,
        connections_file_path: str | None = None,
        config: ConnectionConfig | None = None,
        **kwargs: ConnectionParamValue,
    ) -> None:
        self._loop = get_background_loop()

        # Construction is synchronous — only config parsing + handle allocation
        self._async = AsyncConnection(
            connection_name=connection_name,
            connections_file_path=connections_file_path,
            config=config,
            **kwargs,
        )
        # Authentication is async — run on the background loop
        self._run(self._async.connect())

        self._messages = self._async._messages
        self._errorhandler = self._async._errorhandler
        self.auto_cleanup: bool = self._async.auto_cleanup
        self._interpolate_empty_sequences: bool = self._async._interpolate_empty_sequences
        self._close_lock = threading.Lock()

        if self._should_auto_cleanup():
            atexit.register(self._close_at_process_exit)

    def _run(self, coro: Any) -> Any:
        return asyncio.run_coroutine_threadsafe(coro, self._loop).result()

    # -- delegate properties to AsyncConnection ----------------------------

    @property
    def config(self) -> ConnectionConfig:
        return self._async.config

    @config.setter
    def config(self, value: ConnectionConfig) -> None:
        self._async.config = value

    @property
    def db_handle(self) -> Any:
        return self._async.db_handle

    @db_handle.setter
    def db_handle(self, value: Any) -> None:
        self._async.db_handle = value

    @property
    def conn_handle(self) -> Any:
        return self._async.conn_handle

    @conn_handle.setter
    def conn_handle(self, value: Any) -> None:
        self._async.conn_handle = value

    @property
    def _telemetry_client(self) -> Any:
        return self._async._telemetry_client

    @property
    def _session_parameters(self) -> Any:
        return self._async._session_parameters

    @property
    def _connection_info(self) -> Any:
        return self._async._connection_info

    # -- PEP 249 methods ---------------------------------------------------

    @pep249
    @api_telemetry
    def close(self, retry: bool = True) -> None:
        """Close the connection, send logout, and release handles."""
        atexit.unregister(self._close_at_process_exit)
        self._run(self._async.close(retry=retry))

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
        """Return a new Cursor object using the connection."""
        return cursor_class(self)

    def __enter__(self) -> Connection:
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
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
        return self._async._autocommit

    @_requires_open
    @api_telemetry
    def set_autocommit(self, autocommit: bool) -> None:
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
        return self._autocommit

    @pep249
    def autocommit(self, value: bool) -> None:
        self.set_autocommit(value)

    def is_closed(self) -> bool:
        return bool(self._run(self._async.is_closed()))

    def is_valid(self) -> bool:
        return bool(self._run(self._async.is_valid()))

    def _get_session_parameter(self, name: str) -> str | None:
        return self._async._get_session_parameter(name)

    @property
    def paramstyle(self) -> ParamStyle:
        return self._async.paramstyle

    @paramstyle.setter
    def paramstyle(self, value: str | ParamStyle) -> None:
        self._async.paramstyle = value

    @property
    @backward_compatibility
    def _paramstyle(self) -> ParamStyle:
        return self._async._paramstyle

    @_paramstyle.setter
    @backward_compatibility
    def _paramstyle(self, value: str | ParamStyle) -> None:
        self._async.paramstyle = value

    @api_telemetry
    def execute_string(
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
        from ._internal.text_utils import split_statements

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
        return SnowflakeRestful(connection=self)

    @internal_api
    def _get_connection_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        return cast(
            ConnectionGetInfoResponse,
            self._run(self._async._get_connection_info(include_master_token=include_master_token)),
        )

    @internal_api
    @backward_compatibility
    def _telemetry(self) -> Any:
        from .telemetry import TelemetryClient

        return TelemetryClient()

    @property
    def role(self) -> str | None:
        return self._async.role

    @property
    def database(self) -> str | None:
        return self._async.database

    @property
    def schema(self) -> str | None:
        return self._async.schema

    @property
    def account(self) -> str | None:
        return self._async.account

    @property
    def warehouse(self) -> str | None:
        return self._async.warehouse

    @property
    def user(self) -> str | None:
        return self._async.user

    @property
    def host(self) -> str | None:
        return self._async.host

    @property
    def port(self) -> int | None:
        return self._async.port

    @property
    def session_id(self) -> int:
        return self._async.session_id

    @property
    def login_timeout(self) -> int | None:
        raise NotImplementedError("login_timeout is not yet implemented")

    @property
    def network_timeout(self) -> int | None:
        raise NotImplementedError("network_timeout is not yet implemented")

    @property
    def socket_timeout(self) -> int | None:
        raise NotImplementedError("socket_timeout is not yet implemented")

    @property
    def client_session_keep_alive(self) -> bool | None:
        return self._async.client_session_keep_alive

    @property
    def client_session_keep_alive_heartbeat_frequency(self) -> int | None:
        return self._async.client_session_keep_alive_heartbeat_frequency

    @property
    def client_prefetch_threads(self) -> int:
        raise NotImplementedError("client_prefetch_threads is not yet implemented")

    @client_prefetch_threads.setter
    def client_prefetch_threads(self, value: int) -> None:
        raise NotImplementedError("client_prefetch_threads is not yet implemented")

    @property
    def application(self) -> str:
        return self._async.application

    @property
    @pep249
    def errorhandler(self) -> Callable[..., None]:
        return self._errorhandler

    @errorhandler.setter
    def errorhandler(self, value: Callable[..., None] | None) -> None:
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

    @property
    def _errorhandler_connection(self) -> Connection:
        return self

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    @property
    def is_pyformat(self) -> bool:
        return self._async.is_pyformat

    @property
    def telemetry_enabled(self) -> bool:
        raise NotImplementedError("telemetry_enabled is not yet implemented")

    @telemetry_enabled.setter
    def telemetry_enabled(self, value: bool) -> None:
        raise NotImplementedError("telemetry_enabled is not yet implemented")

    @property
    def service_name(self) -> str | None:
        raise NotImplementedError("service_name is not yet implemented")

    @service_name.setter
    def service_name(self, value: str | None) -> None:
        raise NotImplementedError("service_name is not yet implemented")

    @property
    def log_max_query_length(self) -> int:
        return self._async.log_max_query_length

    def _format_query_for_log(self, query: str) -> str:
        return self._async._format_query_for_log(query)

    @property
    def disable_request_pooling(self) -> bool:
        raise NotImplementedError("disable_request_pooling is not yet implemented")

    @disable_request_pooling.setter
    def disable_request_pooling(self, value: bool) -> None:
        raise NotImplementedError("disable_request_pooling is not yet implemented")

    @property
    def use_openssl_only(self) -> bool:
        raise NotImplementedError("use_openssl_only is not yet implemented")

    @property
    def arrow_number_to_decimal(self) -> bool:
        return self._async.arrow_number_to_decimal

    @arrow_number_to_decimal.setter
    def arrow_number_to_decimal(self, value: bool) -> None:
        self._async.arrow_number_to_decimal = value

    @arrow_number_to_decimal.setter  # type: ignore[attr-defined, untyped-decorator]
    @backward_compatibility
    def arrow_number_to_decimal_setter(self, value: bool) -> None:
        self.arrow_number_to_decimal = value

    @property
    def validate_default_parameters(self) -> bool:
        raise NotImplementedError("validate_default_parameters is not yet implemented")

    @property
    def insecure_mode(self) -> bool:
        raise NotImplementedError("insecure_mode is not yet implemented")

    @property
    def consent_cache_id_token(self) -> bool:
        raise NotImplementedError("consent_cache_id_token is not yet implemented")

    @cached_property
    def snowflake_version(self) -> str:
        with self.cursor(DictCursor) as cur:
            cur.execute("SELECT CURRENT_VERSION() AS version")
            row: dict[str, Any] = cur.fetchone()  # type: ignore[assignment]
        return str(row["VERSION"]).split(" ")[0]

    @api_telemetry
    def get_query_status(self, sf_qid: str) -> QueryStatus:
        return cast(QueryStatus, self._run(self._async.get_query_status(sf_qid)))

    @api_telemetry
    def get_query_status_throw_if_error(self, sf_qid: str) -> QueryStatus:
        return cast(QueryStatus, self._run(self._async.get_query_status_throw_if_error(sf_qid)))

    @staticmethod
    def is_still_running(status: QueryStatus) -> bool:
        return AsyncConnection.is_still_running(status)

    @staticmethod
    def is_an_error(status: QueryStatus) -> bool:
        return AsyncConnection.is_an_error(status)

    # -- cleanup -----------------------------------------------------------

    def _try_close(self) -> None:
        try:
            if not self.is_closed():
                self.close(retry=False)
        except Exception:
            try:
                logger.debug("close() failed during cleanup")
            except Exception:
                pass

    def _should_auto_cleanup(self) -> bool:
        return getattr(self, "auto_cleanup", False)

    def __del__(self) -> None:
        if self._should_auto_cleanup():
            self._try_close()

    def _close_at_process_exit(self) -> None:
        """Cleanup handler called by atexit when process exits.

        Uses a direct synchronous FFI path (bypassing the async event loop)
        because asyncio.to_thread can hang during interpreter shutdown.
        """
        try:
            try:
                warnings.warn(
                    "Connection was not explicitly closed before process exit. "
                    "Auto-cleanup at exit will be disabled by default in a future version. "
                    "Please explicitly call connection.close() or use context manager.",
                    FutureWarning,
                    stacklevel=2,
                )
            except Exception:
                pass

            conn_handle = self.conn_handle
            db_handle = self.db_handle
            self.conn_handle = None
            self.db_handle = None
            if conn_handle is not None:
                connection_close_at_exit(conn_handle, db_handle)
        except Exception:
            try:
                logger.warning("_close_at_process_exit failed during interpreter shutdown")
            except Exception:
                pass


SnowflakeConnection = Connection
