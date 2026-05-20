"""Async-native Connection implementation.

``AsyncConnection`` is the single source of truth for connection logic.
The synchronous :class:`~snowflake.connector.connection.Connection` is a thin
blocking wrapper that delegates to this class.
"""

from __future__ import annotations

import asyncio
import functools
import logging
import platform
import warnings

from collections.abc import AsyncGenerator, Callable
from io import StringIO
from typing import TYPE_CHECKING, Any, TypeVar, cast

from .._internal.api_client.client_api import async_core_driver
from .._internal.binding_converters import ParamStyle
from .._internal.config_utils import create_config_settings_from_dict
from .._internal.decorators import api_telemetry, backward_compatibility, internal_api, pep249
from .._internal.errorcode import ER_CONNECTION_IS_CLOSED, ER_INVALID_VALUE
from .._internal.errorhandler import ErrorHandlerMixin
from .._internal.extras import check_dependency
from .._internal.extras import numpy as np
from .._internal.logout_config_mapping import (
    LogoutOptionKeys,
    logout_config_options_modifier,
)
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
    WrapperIdentity,
)
from .._internal.snowflake_restful import SnowflakeRestful
from .._internal.sqlstate import SQLSTATE_CONNECTION_NOT_EXISTS
from .._internal.text_utils import split_statements
from ..connection_config import ConnectionConfig
from ..constants import QueryStatus
from ..errors import DatabaseError, Error, ErrorValue, InterfaceError, ProgrammingError
from ..version import __version__
from .cursor import AsyncSnowflakeCursor, AsyncSnowflakeCursorBase


if TYPE_CHECKING:
    from .._internal.protobuf_gen.database_driver_v1_services import (
        ConnectionGetInfoResponse,
        ConnectionGetQueryStatusResponse,
    )

_APPLICATION_NAME = "PythonConnector"

LOG_MAX_QUERY_LENGTH = 80

SessionParameters = dict[str, Any]
ConnectionParamValue = int | str | float | bytes | bool | SessionParameters
ConnectionParameters = dict[str, ConnectionParamValue]

logger = logging.getLogger(__name__)


F = TypeVar("F", bound=Callable[..., Any])


def _requires_open(func: F) -> F:
    """Raise ``DatabaseError`` if the connection is closed."""

    @functools.wraps(func)
    async def wrapper(self: AsyncConnection, *args: Any, **kwargs: Any) -> Any:
        if await self.is_closed():
            raise DatabaseError(
                msg="Connection is closed.",
                errno=ER_CONNECTION_IS_CLOSED,
                sqlstate=SQLSTATE_CONNECTION_NOT_EXISTS,
            )
        return await func(self, *args, **kwargs)

    return cast(F, wrapper)


class AsyncTelemetryClient:
    """Telemetry client backed by async_core_driver.

    ``send_api_usage`` / ``send_wrapper_error`` are called synchronously by the
    ``@api_telemetry`` decorator (both on sync and async code paths). They
    submit the coroutine to the background loop and wait for completion so
    callers observe the side effect immediately (important for testability).
    """

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle = conn_handle

    def send_api_usage(self, api_name: str) -> None:
        from .._internal.api_client.client_api import get_background_loop

        try:
            loop = get_background_loop()
            asyncio.run_coroutine_threadsafe(
                async_core_driver.telemetry_send_api_usage(
                    conn_handle=self._conn_handle,
                    api_method=api_name,
                ),
                loop,
            ).result()
        except Exception:
            logger.debug("Failed to send api_usage telemetry", exc_info=True)

    def send_wrapper_error(self, exception_type: str, error_source: str) -> None:
        from .._internal.api_client.client_api import get_background_loop

        try:
            loop = get_background_loop()
            asyncio.run_coroutine_threadsafe(
                async_core_driver.telemetry_send_wrapper_error(
                    conn_handle=self._conn_handle,
                    exception_type=exception_type,
                    error_source=error_source,
                ),
                loop,
            ).result()
        except Exception:
            logger.debug("Failed to send wrapper_error telemetry", exc_info=True)


class _AsyncSessionParametersProxy:
    """Session parameter proxy backed by async_core_driver."""

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle: ConnectionHandle | None = conn_handle
        self._cache: dict[str, str] | None = None

    async def freeze(self) -> None:
        if self._cache is not None:
            return
        handle = cast(ConnectionHandle, self._conn_handle)
        response = await async_core_driver.connection_get_all_parameters(conn_handle=handle)
        self._cache = {k.upper(): v for k, v in response.parameters.items()}
        self._conn_handle = None

    def freeze_sync(self) -> None:
        """Sync freeze for use by the blocking wrapper's close path."""
        if self._cache is not None:
            return
        from .._internal.api_client.client_api import get_background_loop

        loop = get_background_loop()
        asyncio.run_coroutine_threadsafe(self.freeze(), loop).result()

    def __getitem__(self, name: str) -> str | None:
        if self._cache is not None:
            return self._cache.get(name.upper())
        from .._internal.api_client.client_api import get_background_loop

        handle = cast(ConnectionHandle, self._conn_handle)
        loop = get_background_loop()
        response = asyncio.run_coroutine_threadsafe(
            async_core_driver.connection_get_parameter(conn_handle=handle, key=name),
            loop,
        ).result()
        return response.value if response.value else None


class _AsyncConnectionInfoProxy:
    """Connection info proxy backed by async_core_driver."""

    def __init__(self, conn_handle: ConnectionHandle) -> None:
        self._conn_handle: ConnectionHandle | None = conn_handle
        self._cache: dict[str, Any] | None = None

    async def freeze(self) -> None:
        if self._cache is not None:
            return
        handle = cast(ConnectionHandle, self._conn_handle)
        info = await async_core_driver.connection_get_info(
            conn_handle=handle,
            include_master_token=True,
        )
        self._cache = {desc.name: value for desc, value in info.ListFields()}
        self._conn_handle = None

    def freeze_sync(self) -> None:
        if self._cache is not None:
            return
        from .._internal.api_client.client_api import get_background_loop

        loop = get_background_loop()
        asyncio.run_coroutine_threadsafe(self.freeze(), loop).result()

    def __getitem__(self, key: str) -> Any:
        if self._cache is not None:
            return self._cache.get(key)
        from .._internal.api_client.client_api import get_background_loop

        handle = cast(ConnectionHandle, self._conn_handle)
        loop = get_background_loop()
        info = asyncio.run_coroutine_threadsafe(
            async_core_driver.connection_get_info(conn_handle=handle),
            loop,
        ).result()
        return getattr(info, key) if info.HasField(key) else None  # type: ignore[arg-type]


class AsyncConnection(ErrorHandlerMixin):
    """Async-native connection to Snowflake.

    Construction is synchronous — only config parsing and handle allocation
    happen in ``__init__``.  Call :meth:`connect` (or use ``async with``) to
    authenticate and initialise the session.

    The synchronous :class:`~snowflake.connector.connection.Connection` wraps
    this class and calls :meth:`connect` automatically during its own
    ``__init__``.
    """

    def __init__(
        self,
        *,
        connection_name: str | None = None,
        connections_file_path: str | None = None,
        config: ConnectionConfig | None = None,
        **kwargs: ConnectionParamValue,
    ) -> None:
        self.config = ConnectionConfig.from_connection_args(
            connection_name=connection_name,
            connections_file_path=connections_file_path,
            config=config,
            **kwargs,
        )
        if self.config.numpy:
            check_dependency(np)

        from snowflake.connector import paramstyle as default_paramstyle

        self.paramstyle = self.config.paramstyle or default_paramstyle

        self._messages: list[tuple[type[Exception], ErrorValue]] = []
        self._errorhandler: Callable[..., None] = Error.default_errorhandler
        self.db_handle: DatabaseHandle | None = None
        self.conn_handle: ConnectionHandle | None = None
        self._connected = False
        self.auto_cleanup: bool = True if self.config.auto_cleanup is None else bool(self.config.auto_cleanup)
        self._interpolate_empty_sequences: bool = False

        self._session_parameters: _AsyncSessionParametersProxy | None = None
        self._connection_info: _AsyncConnectionInfoProxy | None = None
        self._telemetry_client: AsyncTelemetryClient | None = None

    async def connect(self) -> AsyncConnection:
        """Authenticate and initialise the session (async).

        Allocates database/connection handles, pushes config options into the
        core driver, and runs ``connection_init`` (the auth handshake).
        Returns *self* for convenience (``conn = await AsyncConnection(...).connect()``).
        """
        if self._connected:
            return self

        db_handle = (await async_core_driver.database_new()).db_handle
        await async_core_driver.database_init(db_handle=db_handle)
        conn_handle = (await async_core_driver.connection_new()).conn_handle

        options = self.config.to_proto_options(options_modifiers=[logout_config_options_modifier])
        if options:
            response = await async_core_driver.connection_set_options(conn_handle=conn_handle, options=options)
            for w in response.warnings:
                warnings.warn(w.message, stacklevel=2)

        session_params = self.config.session_parameters
        if session_params:
            await async_core_driver.connection_set_session_parameters(
                conn_handle=conn_handle, parameters=session_params
            )

        self.db_handle = db_handle
        self.conn_handle = conn_handle

        await async_core_driver.connection_init(
            conn_handle=conn_handle,
            db_handle=db_handle,
            wrapper_identity=WrapperIdentity(
                driver_name=_APPLICATION_NAME,
                driver_version=__version__,
                language_runtime=platform.python_implementation(),
                language_version=platform.python_version(),
                language_compiler=platform.python_compiler(),
            ),
        )

        self._telemetry_client = AsyncTelemetryClient(conn_handle=conn_handle)
        self._session_parameters = _AsyncSessionParametersProxy(conn_handle)
        self._connection_info = _AsyncConnectionInfoProxy(conn_handle)
        self._connected = True
        return self

    # -- PEP 249 methods ---------------------------------------------------

    @pep249
    @api_telemetry
    async def close(self, retry: bool = True) -> None:
        """Close the connection, send logout, and release handles."""
        if await self.is_closed():
            return

        if self._session_parameters is not None:
            await self._session_parameters.freeze()
        if self._connection_info is not None:
            await self._connection_info.freeze()

        del self._messages[:]
        conn_handle, self.conn_handle = self.conn_handle, None
        db_handle, self.db_handle = self.db_handle, None

        try:
            if conn_handle:
                if not retry:
                    await async_core_driver.connection_set_options(
                        conn_handle=conn_handle,
                        options=create_config_settings_from_dict({LogoutOptionKeys.LOGOUT_MAX_ATTEMPTS: 1}),
                    )
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

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    @pep249
    @api_telemetry
    @_requires_open
    async def commit(self) -> None:
        """Commit any pending transaction."""
        cur = self.cursor()
        try:
            await cur.execute("COMMIT")
        finally:
            await cur.close()

    @pep249
    @api_telemetry
    @_requires_open
    async def rollback(self) -> None:
        """Roll back to the start of any pending transaction."""
        cur = self.cursor()
        try:
            await cur.execute("ROLLBACK")
        finally:
            await cur.close()

    @pep249
    @api_telemetry
    def cursor(
        self,
        cursor_class: type[AsyncSnowflakeCursorBase] = AsyncSnowflakeCursor,
    ) -> AsyncSnowflakeCursorBase:
        """Return a new async cursor."""
        return cursor_class(self)

    # -- async context manager ---------------------------------------------

    async def __aenter__(self) -> AsyncConnection:
        if not self._connected:
            await self.connect()
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        try:
            if not await self.is_closed() and not self._autocommit:
                if exc_type is None:
                    await self.commit()
                else:
                    try:
                        await self.rollback()
                    except Exception:
                        logger.warning("Rollback failed during exception handling", exc_info=True)
        finally:
            await self.close()

    @property
    def _autocommit(self) -> bool:
        value = self._get_session_parameter("AUTOCOMMIT")
        return value is not None and value.lower() == "true"

    @_requires_open
    @api_telemetry
    async def set_autocommit(self, autocommit: bool) -> None:
        if not isinstance(autocommit, bool):
            raise ProgrammingError(msg=f"Invalid autocommit parameter: {autocommit!r}", errno=ER_INVALID_VALUE)
        cur = self.cursor()
        try:
            await cur.execute(f"ALTER SESSION SET autocommit={str(autocommit).lower()}")
        except Error as e:
            logger.warning("Autocommit feature is not enabled for this connection. Ignored: %s", e)
        finally:
            await cur.close()

    @api_telemetry
    def get_autocommit(self) -> bool:
        return self._autocommit

    @pep249
    async def autocommit(self, value: bool) -> None:
        await self.set_autocommit(value)

    async def is_closed(self) -> bool:
        """Check if the connection is closed."""
        if self.conn_handle is None:
            return True
        try:
            response = await async_core_driver.connection_is_closed(conn_handle=self.conn_handle)
            return bool(response.is_closed)
        except Exception:
            return True

    async def is_valid(self) -> bool:
        """Check whether the connection is still usable."""
        if await self.is_closed():
            return False
        try:
            response = await async_core_driver.connection_heartbeat(
                conn_handle=cast(ConnectionHandle, self.conn_handle),
            )
            return bool(response.valid)
        except Exception:
            return False

    def _get_session_parameter(self, name: str) -> str | None:
        if self._session_parameters is None:
            return None
        return self._session_parameters[name]

    # -- paramstyle --------------------------------------------------------

    @property
    def paramstyle(self) -> ParamStyle:
        return self._paramstyle_value

    @paramstyle.setter
    def paramstyle(self, value: str | ParamStyle) -> None:
        if isinstance(value, ParamStyle):
            self._paramstyle_value = value
        elif isinstance(value, str):
            self._paramstyle_value = ParamStyle.from_string(value)
        else:
            raise ProgrammingError(msg=f"paramstyle must be str or ParamStyle, got {type(value).__name__}")

    @property
    @backward_compatibility
    def _paramstyle(self) -> ParamStyle:
        return self._paramstyle_value

    @_paramstyle.setter
    @backward_compatibility
    def _paramstyle(self, value: str | ParamStyle) -> None:
        self.paramstyle = value

    # -- execute helpers ---------------------------------------------------

    @api_telemetry
    async def execute_string(
        self,
        sql_text: str,
        remove_comments: bool = False,
        return_cursors: bool = True,
        cursor_class: type[AsyncSnowflakeCursorBase] = AsyncSnowflakeCursor,
        **kwargs: Any,
    ) -> list[AsyncSnowflakeCursorBase]:
        """Execute SQL text containing multiple statements."""
        cursors: list[AsyncSnowflakeCursorBase] = []
        async for cur in self.execute_stream(
            StringIO(sql_text), remove_comments=remove_comments, cursor_class=cursor_class
        ):
            if return_cursors:
                cursors.append(cur)
        return cursors

    @api_telemetry
    async def execute_stream(
        self,
        stream: StringIO,
        remove_comments: bool = False,
        cursor_class: type[AsyncSnowflakeCursorBase] = AsyncSnowflakeCursor,
        **kwargs: Any,
    ) -> AsyncGenerator[AsyncSnowflakeCursorBase, None]:
        """Execute a stream of SQL statements."""
        for sql, is_put_or_get in split_statements(stream, remove_comments=remove_comments):
            if not sql:
                continue
            cur = self.cursor(cursor_class=cursor_class)
            await cur.execute(sql, _is_put_get=is_put_or_get)
            yield cur

    # -- internal / backward-compat ----------------------------------------

    @property
    @internal_api
    @backward_compatibility
    def rest(self) -> SnowflakeRestful:
        return SnowflakeRestful(connection=self)  # type: ignore[arg-type]

    @internal_api
    async def _get_connection_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        return await async_core_driver.connection_get_info(
            conn_handle=cast(ConnectionHandle, self.conn_handle),
            include_master_token=include_master_token,
        )

    @internal_api
    @backward_compatibility
    def _telemetry(self) -> Any:
        from ..telemetry import TelemetryClient

        return TelemetryClient()

    # -- connection info properties ----------------------------------------

    def _info(self, key: str) -> Any:
        if self._connection_info is None:
            return None
        return self._connection_info[key]

    @property
    def role(self) -> str | None:
        return cast("str | None", self._info("role"))

    @property
    def database(self) -> str | None:
        return cast("str | None", self._info("database"))

    @property
    def schema(self) -> str | None:
        return cast("str | None", self._info("schema"))

    @property
    def account(self) -> str | None:
        return cast("str | None", self._info("account"))

    @property
    def warehouse(self) -> str | None:
        return cast("str | None", self._info("warehouse"))

    @property
    def user(self) -> str | None:
        return cast("str | None", self._info("user"))

    @property
    def host(self) -> str | None:
        return cast("str | None", self._info("host"))

    @property
    def port(self) -> int | None:
        return cast("int | None", self._info("port"))

    @property
    def session_id(self) -> int:
        value = cast("int | None", self._info("session_id"))
        if value is None:
            raise InterfaceError(msg="Session ID is not available; connection may not be initialized")
        return value

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
        return self.config.client_session_keep_alive

    @property
    def client_session_keep_alive_heartbeat_frequency(self) -> int | None:
        return self.config.client_session_keep_alive_heartbeat_frequency

    @property
    def client_prefetch_threads(self) -> int:
        raise NotImplementedError("client_prefetch_threads is not yet implemented")

    @client_prefetch_threads.setter
    def client_prefetch_threads(self, value: int) -> None:
        raise NotImplementedError("client_prefetch_threads is not yet implemented")

    @property
    def application(self) -> str:
        return self.config.application  # type: ignore[return-value]

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
    def _errorhandler_connection(self) -> AsyncConnection:  # type: ignore[override]
        return self

    @property
    def is_pyformat(self) -> bool:
        return self._paramstyle in (ParamStyle.PYFORMAT, ParamStyle.FORMAT)

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
        return (
            self.config.log_max_query_length if self.config.log_max_query_length is not None else LOG_MAX_QUERY_LENGTH
        )

    def _format_query_for_log(self, query: str) -> str:
        ret = " ".join(line.strip() for line in query.split("\n"))
        if len(ret) < self.log_max_query_length:
            return ret
        return ret[: self.log_max_query_length] + "..."

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
        return bool(self.config.arrow_number_to_decimal)

    @arrow_number_to_decimal.setter
    def arrow_number_to_decimal(self, value: bool) -> None:
        self.config.arrow_number_to_decimal = bool(value)

    @property
    def validate_default_parameters(self) -> bool:
        raise NotImplementedError("validate_default_parameters is not yet implemented")

    @property
    def insecure_mode(self) -> bool:
        raise NotImplementedError("insecure_mode is not yet implemented")

    @property
    def consent_cache_id_token(self) -> bool:
        raise NotImplementedError("consent_cache_id_token is not yet implemented")

    # -- query status ------------------------------------------------------

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
        from .._internal.protobuf_gen.database_driver_v1_pb2 import ConnectionGetQueryStatusResponse

        if await self.is_closed():
            return QueryStatus.DISCONNECTED, ConnectionGetQueryStatusResponse()
        response = await async_core_driver.connection_get_query_status(
            conn_handle=cast(ConnectionHandle, self.conn_handle), query_id=sf_qid
        )
        try:
            status = QueryStatus[response.status_name]
        except KeyError:
            logger.warning("Unknown query status %r; treating as NO_DATA", response.status_name)
            status = QueryStatus.NO_DATA
        return status, response

    @staticmethod
    def is_still_running(status: QueryStatus) -> bool:
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
        return status in (
            QueryStatus.ABORTING,
            QueryStatus.FAILED_WITH_ERROR,
            QueryStatus.ABORTED,
            QueryStatus.FAILED_WITH_INCIDENT,
            QueryStatus.DISCONNECTED,
        )
