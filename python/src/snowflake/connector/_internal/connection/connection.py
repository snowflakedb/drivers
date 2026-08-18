"""Shared connection mixin for sync and async connection implementations."""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING, Any, ClassVar, Generic, TypeVar, cast

from ...connection_config import ConnectionConfig
from ...constants import QueryStatus, SessionParameterName
from ...errors import Error, ErrorValue, InterfaceError, ProgrammingError
from ..api_client.client_api import core_driver
from ..binding_converters import ParamStyle
from ..decorators import api_telemetry, backward_compatibility, internal_api, pep249
from ..errorhandler import ErrorHandlerMixin
from ..extras import check_dependency
from ..extras import numpy as np
from ..logging import get_logger
from .constants import LOG_MAX_QUERY_LENGTH
from .decorators import requires_open


if TYPE_CHECKING:
    from ...aio.cursor import CursorType as AsyncCursorType
    from ...cursor import CursorType as SyncCursorType
    from ..protobuf_gen.database_driver_v1_pb2 import ConnectionHandle, DatabaseHandle
    from ..protobuf_gen.database_driver_v1_services import ConnectionGetInfoResponse


# Cursor instance type produced by ``cursor()``; each concrete Connection binds
# it to its own (sync vs. async) ``CursorInstance`` union.
_CursorT = TypeVar("_CursorT")

logger = get_logger(__name__)

# Matches the legacy connector's `MAX_CLIENT_PREFETCH_THREADS` /
# `_validate_client_prefetch_threads` bounds (`connection.py`).
MIN_CLIENT_PREFETCH_THREADS = 1
MAX_CLIENT_PREFETCH_THREADS = 10


def clamp_client_prefetch_threads(value: int) -> int:
    """Clamp to ``[1, 10]``, matching the legacy connector's validation."""
    value = int(value)
    if value <= 0:
        return MIN_CLIENT_PREFETCH_THREADS
    if value > MAX_CLIENT_PREFETCH_THREADS:
        return MAX_CLIENT_PREFETCH_THREADS
    return value


class ConnectionMixin(ErrorHandlerMixin, Generic[_CursorT]):
    """Connection members shared by sync and async connection classes.

    Subclasses set handle/proxy fields (``conn_handle``, ``_session_parameters``,
    ``_connection_info``, etc.) and call :meth:`__init__` before performing any
    I/O to establish the session.
    """

    _messages: list[tuple[type[Exception], ErrorValue]]
    _errorhandler: Callable[..., None]
    config: ConnectionConfig
    __paramstyle: ParamStyle
    _interpolate_empty_sequences: bool
    _session_parameters: Any
    _connection_info: Any
    _client_param_telemetry_enabled: bool
    conn_handle: ConnectionHandle | None

    # Concrete subclasses set the default cursor class returned by ``cursor()``
    # (the sync vs. async ``SnowflakeCursor``); kept off the shared mixin so
    # ``_internal`` never imports the public cursor packages at runtime.
    _default_cursor_class: ClassVar[SyncCursorType | AsyncCursorType]

    def __init__(
        self,
        *,
        connection_name: str | None = None,
        connections_file_path: str | None = None,
        config: ConnectionConfig | None = None,
        **kwargs: Any,
    ) -> None:
        self._messages = []
        self._errorhandler = Error.default_errorhandler
        self._client_param_telemetry_enabled = True

        self.config = ConnectionConfig.from_connection_args(
            connection_name=connection_name,
            connections_file_path=connections_file_path,
            config=config,
            **kwargs,
        )

        if self.config.client_prefetch_threads is not None:
            self.config.client_prefetch_threads = clamp_client_prefetch_threads(self.config.client_prefetch_threads)

        from snowflake.connector import paramstyle as default_paramstyle

        self.paramstyle = self.config.paramstyle or default_paramstyle

        if self.config.numpy:
            check_dependency(np)

        self._interpolate_empty_sequences = False

    # ------------------------------------------------------------------
    # Cursors
    # ------------------------------------------------------------------

    @pep249
    @api_telemetry
    @requires_open
    def cursor(self, cursor_class: type[_CursorT] | None = None) -> _CursorT:
        """Return a new Cursor object using the connection.

        Args:
            cursor_class: The class to instantiate. Defaults to the
                connection's ``SnowflakeCursor``; pass ``DictCursor`` to get
                results as dictionaries.

        Returns:
            A new cursor object.
        """
        # The concrete Connection subclass passes itself to the cursor
        # constructor; the mixin can't prove ``self`` is that concrete type,
        # so it builds through an untyped factory.
        factory: Any = cursor_class or self._default_cursor_class
        return cast("_CursorT", factory(self))

    # ------------------------------------------------------------------
    # Handle release
    # ------------------------------------------------------------------

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

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
    @pep249
    @api_telemetry
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    @api_telemetry
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    # ------------------------------------------------------------------
    # Paramstyle
    # ------------------------------------------------------------------

    @property
    @api_telemetry
    def paramstyle(self) -> ParamStyle:
        """Get the paramstyle for this connection."""
        return self.__paramstyle

    @paramstyle.setter
    @api_telemetry
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

    # ------------------------------------------------------------------
    # Session info
    # ------------------------------------------------------------------

    @property
    @api_telemetry
    def role(self) -> str | None:
        """The current role in use for the session."""
        return cast("str | None", self._connection_info["role"])

    @property
    @api_telemetry
    def database(self) -> str | None:
        """The current database in use for the session."""
        return cast("str | None", self._connection_info["database"])

    @property
    @api_telemetry
    def schema(self) -> str | None:
        """The current schema in use for the session."""
        return cast("str | None", self._connection_info["schema"])

    @property
    @api_telemetry
    def account(self) -> str | None:
        """The Snowflake account name used by this connection."""
        return cast("str | None", self._connection_info["account"])

    @property
    @api_telemetry
    def warehouse(self) -> str | None:
        """The current warehouse in use for the session."""
        return cast("str | None", self._connection_info["warehouse"])

    @property
    @api_telemetry
    def user(self) -> str | None:
        """The user name used for authentication."""
        return cast("str | None", self._connection_info["user"])

    @property
    @api_telemetry
    def host(self) -> str | None:
        """The host name of the Snowflake instance."""
        return cast("str | None", self._connection_info["host"])

    @property
    @api_telemetry
    def port(self) -> int | None:
        """The port number of the Snowflake instance."""
        return cast("int | None", self._connection_info["port"])

    @property
    @api_telemetry
    def session_id(self) -> int:
        """The Snowflake session ID for this connection."""
        value = cast("int | None", self._connection_info["session_id"])
        if value is None:
            raise InterfaceError(msg="Session ID is not available; connection may not be initialized")
        return value

    # ------------------------------------------------------------------
    # Proxy info
    # ------------------------------------------------------------------

    @property
    @api_telemetry
    def proxy_host(self) -> str | None:
        """The configured HTTP proxy hostname, or ``None`` when no proxy is set."""
        return cast("str | None", self._connection_info["proxy_host"])

    @property
    @api_telemetry
    def proxy_port(self) -> int | None:
        """The configured HTTP proxy port, or ``None`` when no proxy is set."""
        return cast("int | None", self._connection_info["proxy_port"])

    @property
    @api_telemetry
    def proxy_user(self) -> str | None:
        """The configured HTTP proxy username for Basic auth, or ``None`` when unset."""
        return cast("str | None", self._connection_info["proxy_user"])

    @property
    @api_telemetry
    def proxy_password(self) -> str | None:
        """The configured HTTP proxy password for Basic auth, or ``None`` when unset."""
        return cast("str | None", self._connection_info["proxy_password"])

    @property
    @api_telemetry
    def no_proxy(self) -> str | None:
        """Comma-separated list of hosts that bypass the proxy, or ``None`` when unset."""
        return cast("str | None", self._connection_info["no_proxy"])

    # ------------------------------------------------------------------
    # Connection properties
    # ------------------------------------------------------------------

    @property
    @api_telemetry
    def login_timeout(self) -> int | None:
        """The login timeout in seconds."""
        return self.config.login_timeout

    @property
    @api_telemetry
    def network_timeout(self) -> int | None:
        """The network timeout in seconds for all other operations."""
        return self.config.request_timeout

    @property
    @api_telemetry
    def socket_timeout(self) -> int | None:
        """The socket timeout in seconds."""
        return self.config.retry_timeout

    @property
    @api_telemetry
    def client_session_keep_alive(self) -> bool | None:
        """Whether to keep the session active with periodic heartbeat requests."""
        return self.config.client_session_keep_alive

    @property
    @api_telemetry
    def client_session_keep_alive_heartbeat_frequency(self) -> int | None:
        """The frequency in seconds of heartbeat requests when session keep-alive is enabled."""
        return self.config.client_session_keep_alive_heartbeat_frequency

    @property
    @api_telemetry
    def client_prefetch_threads(self) -> int | None:
        """The number of threads used to prefetch query result data."""
        return self.config.client_prefetch_threads

    @client_prefetch_threads.setter
    @api_telemetry
    def client_prefetch_threads(self, value: int) -> None:
        """Update local state only; this is the base (zero-I/O) implementation.

        The core's chunk-prefetch pool size is read from the server-echoed
        session-parameter cache, not from this config value, so on its own
        this setter has no effect on an already-open connection. The
        synchronous ``Connection`` class overrides this property to also run
        ``ALTER SESSION SET CLIENT_PREFETCH_THREADS`` so the change actually
        takes effect on subsequent fetches, matching the old connector's
        immediate, locally-effective setter. Async connections cannot do the
        same through a synchronous property setter (it can't ``await``); use
        ``await conn.set_client_prefetch_threads(value)`` there instead.
        """
        self.config.client_prefetch_threads = clamp_client_prefetch_threads(value)

    @property
    @api_telemetry
    def application(self) -> str:
        """The name of the client application connecting to Snowflake."""
        return self.config.application  # type: ignore[return-value]

    # ------------------------------------------------------------------
    # Error handling
    # ------------------------------------------------------------------

    @property
    @pep249
    @api_telemetry
    def errorhandler(self) -> Callable:
        """PEP 249 error handler called for connection and cursor errors."""
        return self._errorhandler

    @errorhandler.setter
    @api_telemetry
    def errorhandler(self, value: Callable | None) -> None:
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

    @property
    @api_telemetry
    def is_pyformat(self) -> bool:
        """Whether the connection uses pyformat or format paramstyle (client-side binding)."""
        return self._paramstyle in (ParamStyle.PYFORMAT, ParamStyle.FORMAT)

    # ------------------------------------------------------------------
    # Logging & options
    # ------------------------------------------------------------------

    def _server_param_telemetry_enabled(self) -> bool:
        try:
            value = self._session_parameters[SessionParameterName.CLIENT_TELEMETRY_ENABLED]
        except Exception:
            logger.debug("Failed to read CLIENT_TELEMETRY_ENABLED session parameter")
            return False
        return value is not None and value.lower() == "true"

    @property
    @api_telemetry
    def telemetry_enabled(self) -> bool:
        """Whether client-side telemetry collection is enabled.

        True only when both halves agree: the client has not disabled it via
        this property, AND the server has confirmed ``CLIENT_TELEMETRY_ENABLED``
        for the session. Unlike ``sf_core``'s own (unrelated) OTEL-export gate,
        an unconfirmed server parameter is treated as disabled, not enabled,
        matching the old driver version. Reading this property issues an RPC.
        """
        return self._client_param_telemetry_enabled and self._server_param_telemetry_enabled()

    @telemetry_enabled.setter
    @api_telemetry
    def telemetry_enabled(self, value: bool) -> None:
        """Set the client-side telemetry flag.

        This can only narrow, never widen, server policy: it never issues an
        RPC or ``ALTER SESSION``.
        """
        self._client_param_telemetry_enabled = bool(value)
        if self._client_param_telemetry_enabled and not self._server_param_telemetry_enabled():
            logger.info(
                "Telemetry has been disabled by the session parameter CLIENT_TELEMETRY_ENABLED."
                " Set session parameter CLIENT_TELEMETRY_ENABLED to true to enable telemetry."
            )

    @property
    @api_telemetry
    def service_name(self) -> str | None:
        """The Snowflake service name for the connection, used for service discovery."""
        raise NotImplementedError("service_name is not yet implemented")

    @service_name.setter
    @api_telemetry
    def service_name(self, value: str | None) -> None:
        raise NotImplementedError("service_name is not yet implemented")

    @property
    @api_telemetry
    def log_max_query_length(self) -> int:
        """Maximum number of characters of a query string to log."""
        return (
            self.config.log_max_query_length if self.config.log_max_query_length is not None else LOG_MAX_QUERY_LENGTH
        )

    def _format_query_for_log(self, query: str) -> str:
        """Collapse whitespace and truncate a query string for gated INFO query logging."""
        ret = " ".join(line.strip() for line in query.split("\n"))
        if len(ret) < self.log_max_query_length:
            return ret
        return ret[: self.log_max_query_length] + "..."

    @property
    @api_telemetry
    def disable_request_pooling(self) -> bool:
        """Whether HTTP connection pooling is disabled."""
        raise NotImplementedError("disable_request_pooling is not yet implemented")

    @disable_request_pooling.setter
    @api_telemetry
    def disable_request_pooling(self, value: bool) -> None:
        raise NotImplementedError("disable_request_pooling is not yet implemented")

    @property
    @api_telemetry
    def use_openssl_only(self) -> bool:
        """Deprecated. Whether to restrict TLS to OpenSSL only (always ``True``)."""
        raise NotImplementedError("use_openssl_only is not yet implemented")

    @property
    @api_telemetry
    def arrow_number_to_decimal(self) -> bool:
        """Whether to convert Arrow numeric types to Python ``Decimal`` instead of ``float``."""
        return bool(self.config.arrow_number_to_decimal)

    @arrow_number_to_decimal.setter
    @api_telemetry
    def arrow_number_to_decimal(self, value: bool) -> None:
        self.config.arrow_number_to_decimal = bool(value)

    @arrow_number_to_decimal.setter  # type: ignore[attr-defined, untyped-decorator]
    @backward_compatibility
    def arrow_number_to_decimal_setter(self, value: bool) -> None:
        """Set arrow_number_to_decimal field. Deprecated."""
        self.arrow_number_to_decimal = value

    @property
    @api_telemetry
    def validate_default_parameters(self) -> bool:
        """Whether to validate default connection parameters at connect time."""
        return bool(self.config.validate_default_parameters)

    @property
    @api_telemetry
    def insecure_mode(self) -> bool:
        """Whether OCSP certificate revocation checking is disabled."""
        raise NotImplementedError("insecure_mode is not yet implemented")

    @property
    @api_telemetry
    def consent_cache_id_token(self) -> bool:
        """Whether to cache the IdP token for browser-based SSO authentication."""
        raise NotImplementedError("consent_cache_id_token is not yet implemented")

    # ------------------------------------------------------------------
    # connection state (sync core_driver reads)
    # ------------------------------------------------------------------

    def _autocommit_enabled(self) -> bool:
        value = self._session_parameters["AUTOCOMMIT"]
        return value is not None and value.lower() == "true"

    @property
    def _autocommit(self) -> bool:
        """Whether autocommit is enabled (legacy internal name used by sync tests)."""
        return self._autocommit_enabled()

    @api_telemetry
    def get_autocommit(self) -> bool:
        """Return the current autocommit mode."""
        return self._autocommit_enabled()

    @api_telemetry
    def get_client_prefetch_threads(self) -> int:
        """Return the configured number of chunk-prefetch threads."""
        return cast(int, self.config.client_prefetch_threads)

    @api_telemetry
    def is_expired(self) -> bool:
        """Return True if the connection's master token has expired.

        Once True, the session can no longer be renewed and the connection
        must be replaced; full re-authentication is required.

        Matches the legacy ``SnowflakeConnection.expired`` flag — intended as a
        read-only signal for external pool / application code. Fails closed on
        RPC errors so pools evict uncertain connections.
        """
        if self.conn_handle is None:
            return False
        try:
            response = core_driver.connection_is_expired(conn_handle=self.conn_handle)
            return bool(response.is_expired)
        except Exception:
            return True

    @property
    @api_telemetry
    def expired(self) -> bool:
        """Whether the master token has expired (sync legacy property name)."""
        return self.is_expired()

    @internal_api
    def _get_connection_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        """Return connection details from Core."""
        if self.conn_handle is None:
            raise InterfaceError(msg="Connection handle is not available")
        return core_driver.connection_get_info(
            conn_handle=self.conn_handle,
            include_master_token=include_master_token,
        )

    # ------------------------------------------------------------------
    # Query status helpers
    # ------------------------------------------------------------------

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
