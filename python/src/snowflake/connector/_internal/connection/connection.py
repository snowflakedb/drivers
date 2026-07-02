"""Shared connection mixin for sync and async connection implementations."""

from __future__ import annotations

from collections.abc import Callable
from typing import Any, cast

from ...connection_config import ConnectionConfig
from ...constants import QueryStatus
from ...errors import Error, ErrorValue, InterfaceError, ProgrammingError
from ..binding_converters import ParamStyle
from ..decorators import backward_compatibility, pep249
from ..errorhandler import ErrorHandlerMixin
from ..extras import check_dependency
from ..extras import numpy as np
from .constants import LOG_MAX_QUERY_LENGTH


class ConnectionMixin(ErrorHandlerMixin):
    """Zero-I/O connection members shared by sync and async connection classes.

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

        self.config = ConnectionConfig.from_connection_args(
            connection_name=connection_name,
            connections_file_path=connections_file_path,
            config=config,
            **kwargs,
        )

        from snowflake.connector import paramstyle as default_paramstyle

        self.paramstyle = self.config.paramstyle or default_paramstyle

        if self.config.numpy:
            check_dependency(np)

        self._interpolate_empty_sequences = False

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    # ------------------------------------------------------------------
    # Paramstyle
    # ------------------------------------------------------------------

    @property
    def paramstyle(self) -> ParamStyle:
        """Get the paramstyle for this connection."""
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

    # ------------------------------------------------------------------
    # Session info
    # ------------------------------------------------------------------

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
    def session_id(self) -> int:
        """The Snowflake session ID for this connection."""
        value = cast("int | None", self._connection_info["session_id"])
        if value is None:
            raise InterfaceError(msg="Session ID is not available; connection may not be initialized")
        return value

    # ------------------------------------------------------------------
    # Connection properties
    # ------------------------------------------------------------------

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
        return self.config.client_session_keep_alive

    @property
    def client_session_keep_alive_heartbeat_frequency(self) -> int | None:
        """The frequency in seconds of heartbeat requests when session keep-alive is enabled."""
        return self.config.client_session_keep_alive_heartbeat_frequency

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
        return self.config.application  # type: ignore[return-value]

    # ------------------------------------------------------------------
    # Error handling
    # ------------------------------------------------------------------

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

    # ------------------------------------------------------------------
    # Logging & options
    # ------------------------------------------------------------------

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
        """Set arrow_number_to_decimal field. Deprecated."""
        self.arrow_number_to_decimal = value

    @property
    def validate_default_parameters(self) -> bool:
        """Whether to validate default connection parameters at connect time."""
        return bool(self.config.validate_default_parameters)

    @property
    def insecure_mode(self) -> bool:
        """Whether OCSP certificate revocation checking is disabled."""
        raise NotImplementedError("insecure_mode is not yet implemented")

    @property
    def consent_cache_id_token(self) -> bool:
        """Whether to cache the IdP token for browser-based SSO authentication."""
        raise NotImplementedError("consent_cache_id_token is not yet implemented")

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
