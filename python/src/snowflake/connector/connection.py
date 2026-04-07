"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

import logging
import re

from collections.abc import Generator, Iterable
from functools import cached_property
from io import StringIO
from typing import Any, Callable, Union

from snowflake.connector._internal.errorcode import ER_CONNECTION_IS_CLOSED
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConfigSetting,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionGetParameterRequest,
    ConnectionGetQueryStatusRequest,
    ConnectionGetQueryStatusResponse,
    ConnectionInitRequest,
    ConnectionNewRequest,
    ConnectionSetOptionsRequest,
    ConnectionSetSessionParametersRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
)
from snowflake.connector._internal.snowflake_restful import SnowflakeRestful

from ._internal._private_key_helper import normalize_private_key
from ._internal.api_client.client_api import database_driver_client
from ._internal.binding_converters import ParamStyle
from ._internal.decorators import backward_compatibility, internal_api, pep249
from ._internal.text_utils import split_statements
from .constants import QueryStatus
from .cursor import CursorInstance, CursorType, DictCursor, SnowflakeCursor
from .errors import Error, InterfaceError, NotSupportedError, ProgrammingError
from .telemetry import TelemetryClient


logger = logging.getLogger(__name__)

CLIENT_NAME = "PythonConnector"
# The old connector used re.match(r"[\w\d_]+") without anchors, so any string
# starting with a word character was accepted (dots, hyphens, etc. in the tail
# were silently ignored).  We keep a start-anchored pattern without $ so that
# callers like Snow CLI can pass dotted names such as "SNOWCLI.STAGE.COPY".
APPLICATION_RE = re.compile(r"^[\w\d_]+")

SessionParameters = dict[str, Any]
ConnectionParamValue = Union[int, str, float, bytes, bool, SessionParameters]
ConnectionParameters = dict[str, ConnectionParamValue]


class Connection:
    """Connection objects represent a database connection."""

    def __init__(
        self,
        *,
        paramstyle: str | None = None,
        autocommit: bool | None = None,
        **kwargs: ConnectionParamValue,
    ) -> None:
        """
        Initialize a new connection object.

        Args:
            paramstyle: Binding style – ``"pyformat"`` (default), ``"format"``, ``"qmark"`` or ``"numeric"``
            autocommit: Optional bool to enable/disable autocommit at connection time
            database: Database name
            user: Username
            password: Password
            host: Host name
            port: Port number
            private_key: Private key in bytes, str (base64), or RSAPrivateKey format
            session_parameters: Optional dict of session parameters to set at connection time
            authenticator: Authentication method. Use ``"USERNAME_PASSWORD_MFA"`` for MFA authentication.
            passcode: MFA passcode (TOTP one-time code from an authenticator app). When provided
                with ``authenticator="USERNAME_PASSWORD_MFA"``, the driver automatically uses the
                Duo passcode flow; you do not need to set ``ext_authn_duo_method="passcode"``
                explicitly.
            passcode_in_password: If ``True``, the MFA passcode is appended to the password field
                rather than sent separately. This is treated the same as supplying ``passcode``
                directly and will automatically select the Duo passcode flow. Default ``False``.
            client_store_temporary_credential: If ``True``, a successfully obtained MFA token is
                cached in the OS keyring and reused for subsequent connections, avoiding repeated
                MFA prompts. Default ``False``. The server must have ``ALLOW_CLIENT_MFA_CACHING``
                enabled. This also implicitly requests an MFA token from the server
                (``CLIENT_REQUEST_MFA_TOKEN``).
            client_request_mfa_token: Deprecated alias for ``client_store_temporary_credential``
                from ``snowflake-connector-python``. Accepted for backward compatibility; prefer
                ``client_store_temporary_credential`` in new code.
            ext_authn_duo_method: DUO Security authentication method applied when no explicit
                passcode is provided and no cached MFA token is available. Either ``"push"``
                (send a push notification to the registered device) or ``"passcode"`` (prompt
                for or use a TOTP code). When a ``passcode`` is supplied directly this parameter
                is ignored because the passcode flow is selected automatically.
            **kwargs: Additional connection parameters
        """
        # paramstyle
        from snowflake.connector import paramstyle as default_paramstyle

        self._paramstyle = ParamStyle.from_string(paramstyle or default_paramstyle)

        kwargs = self._rewrite_private_key_password(kwargs)
        kwargs = self._rewrite_mfa_params(kwargs)

        application = kwargs.pop("application", None)
        if application is None or (isinstance(application, str) and not application):
            self._application = CLIENT_NAME
        elif isinstance(application, str):
            if not APPLICATION_RE.match(application):
                raise ProgrammingError(f"Invalid application name: {application!r}")
            self._application = application
        else:
            raise ProgrammingError(f"Invalid application parameter (must be a non-empty string): {application!r}")
        kwargs["client_app_id"] = self._application

        self.db_api = database_driver_client()
        self.db_handle = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle = self.db_api.connection_new(ConnectionNewRequest()).conn_handle

        # Extract session_parameters before processing other kwargs
        session_params: SessionParameters | None = kwargs.pop("session_parameters", None)  # type: ignore

        if autocommit is not None:
            if not isinstance(autocommit, bool):
                raise ProgrammingError(f"Invalid autocommit parameter: {autocommit!r}")

        if session_params is None:
            session_params = {}
        if autocommit is not None:
            session_params["AUTOCOMMIT"] = str(autocommit).lower()

        # Pre-process private_key if present - normalize for Rust core
        if "private_key" in kwargs:
            kwargs["private_key"] = normalize_private_key(kwargs["private_key"])

        options = {}
        for key, value in kwargs.items():
            if isinstance(value, bool):
                options[key] = ConfigSetting(bool_value=value)
            elif isinstance(value, int):
                options[key] = ConfigSetting(int_value=value)
            elif isinstance(value, str):
                options[key] = ConfigSetting(string_value=value)
            elif isinstance(value, float):
                options[key] = ConfigSetting(double_value=value)
            elif isinstance(value, bytes):
                options[key] = ConfigSetting(bytes_value=value)

        if options:
            import warnings as py_warnings

            response = self.db_api.connection_set_options(
                ConnectionSetOptionsRequest(
                    conn_handle=self.conn_handle,
                    options=options,
                )
            )
            for warning in response.warnings:
                py_warnings.warn(warning.message, stacklevel=2)

        # Set session parameters if provided (before connection_init)
        if session_params:
            self.db_api.connection_set_session_parameters(
                ConnectionSetSessionParametersRequest(conn_handle=self.conn_handle, parameters=session_params)
            )

        self.db_api.connection_init(ConnectionInitRequest(conn_handle=self.conn_handle, db_handle=self.db_handle))
        _sensitive_keys = {"password", "private_key", "passcode", "private_key_password", "private_key_file_pwd"}
        self.kwargs = {k: ("***" if k in _sensitive_keys else v) for k, v in kwargs.items()}
        self._closed = False
        self._messages: list[tuple[type[Exception], dict[str, str | bool]]] = []
        self._errorhandler: Callable

        # other connection properties
        self._arrow_number_to_decimal: bool = False

    @pep249
    def close(self) -> None:
        """Close the connection now."""
        self._closed = True

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
        """Commit any pending transaction to the database."""
        cur = self.cursor()
        try:
            cur.execute("COMMIT")
        finally:
            cur.close()

    @pep249
    def rollback(self) -> None:
        """Roll back to the start of any pending transaction."""
        cur = self.cursor()
        try:
            cur.execute("ROLLBACK")
        finally:
            cur.close()

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
        self._check_not_closed()
        return cursor_class(self)

    def _check_not_closed(self) -> None:
        if self._closed:
            raise InterfaceError("Connection is closed.", errno=ER_CONNECTION_IS_CLOSED)

    # Context manager support
    def __enter__(self) -> Connection:
        """
        Enter the runtime context for the connection.

        Returns:
            Connection: Self
        """
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit the runtime context. Commit on success / rollback on exception if autocommit is OFF."""
        try:
            if not self._autocommit and not self._closed:
                if exc_type is None:
                    self.commit()
                else:
                    try:
                        self.rollback()
                    except Exception:
                        logger.warning("Rollback failed during exception handling", exc_info=True)
        finally:
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

    @property
    def _autocommit(self) -> bool:
        value = self._get_session_parameter("AUTOCOMMIT")
        return value is not None and value.lower() == "true"

    def set_autocommit(self, autocommit: bool) -> None:
        """Set the autocommit mode. Executes ALTER SESSION SET autocommit on the server."""
        if not isinstance(autocommit, bool):
            raise ProgrammingError(f"Invalid autocommit parameter: {autocommit!r}")
        cur = self.cursor()
        try:
            cur.execute(f"ALTER SESSION SET autocommit={str(autocommit).lower()}")
        # TODO: Narrow exception handling once proper error propagation is implemented
        except Error as e:
            logger.warning("Autocommit feature is not enabled for this connection. Ignored: %s", e)
        finally:
            cur.close()

    def get_autocommit(self) -> bool:
        """
        Get the current autocommit mode.

        Returns:
            bool: Current autocommit setting
        """
        return self._autocommit

    @pep249
    def autocommit(self, value: bool) -> None:
        """Set autocommit mode."""
        self.set_autocommit(value)

    def is_closed(self) -> bool:
        """
        Check if the connection is closed.

        Returns:
            bool: True if connection is closed, False otherwise
        """
        return self._closed

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

    @backward_compatibility
    def _rewrite_mfa_params(self, kwargs: ConnectionParameters) -> ConnectionParameters:
        """Translate Python-style MFA parameter names to the keys expected by the Rust core.

        Handles two rewrite rules:

        * ``passcode_in_password`` → ``passcodeInPassword`` (camelCase key required by Rust core).
        * ``client_request_mfa_token`` → ``client_store_temporary_credential`` for compatibility
          with ``snowflake-connector-python``, which used the former name for MFA token caching.
          If both are supplied, ``client_store_temporary_credential`` takes precedence and the
          legacy key is discarded.
        """
        passcode_in_password = kwargs.pop("passcode_in_password", None)
        if passcode_in_password is not None:
            kwargs = {**kwargs, "passcodeInPassword": passcode_in_password}

        # client_request_mfa_token is the legacy snowflake-connector-python name for MFA token
        # caching.  Map it to the canonical key so callers migrating from the old driver do not
        # need to update their code.
        legacy_token_cache = kwargs.pop("client_request_mfa_token", None)
        if legacy_token_cache is not None and "client_store_temporary_credential" not in kwargs:
            kwargs = {**kwargs, "client_store_temporary_credential": legacy_token_cache}

        return kwargs

    @property
    def role(self) -> str | None:
        """The current role in use for the session."""
        info = self._get_connection_info()
        return info.role if info.HasField("role") else None

    @property
    def database(self) -> str | None:
        """The current database in use for the session."""
        info = self._get_connection_info()
        return info.database if info.HasField("database") else None

    @property
    def schema(self) -> str | None:
        """The current schema in use for the session."""
        info = self._get_connection_info()
        return info.schema if info.HasField("schema") else None

    @property
    def account(self) -> str | None:
        """The Snowflake account name used by this connection."""
        info = self._get_connection_info()
        return info.account if info.HasField("account") else None

    @property
    def warehouse(self) -> str | None:
        """The current warehouse in use for the session."""
        info = self._get_connection_info()
        return info.warehouse if info.HasField("warehouse") else None

    @property
    def user(self) -> str | None:
        """The user name used for authentication."""
        info = self._get_connection_info()
        return info.user if info.HasField("user") else None

    @property
    def host(self) -> str | None:
        """The host name of the Snowflake instance."""
        info = self._get_connection_info()
        return info.host if info.HasField("host") else None

    @property
    def port(self) -> int | None:
        """The port number of the Snowflake instance."""
        info = self._get_connection_info()
        return info.port if info.HasField("port") else None

    @property
    def region(self) -> str | None:
        """Deprecated. The region for the Snowflake account."""
        raise NotImplementedError("region is not implemented")

    @property
    def session_id(self) -> int:
        """The Snowflake session ID for this connection."""
        info = self._get_connection_info()
        if not info.HasField("session_id"):
            raise InterfaceError("Session ID is not available; connection may not be initialized")
        return info.session_id

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
        return self._application

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
        return self._arrow_number_to_decimal

    @arrow_number_to_decimal.setter
    def arrow_number_to_decimal(self, value: bool) -> None:
        self._arrow_number_to_decimal = bool(value)

    @backward_compatibility
    @arrow_number_to_decimal.setter  # type: ignore[attr-defined, untyped-decorator]
    def arrow_number_to_decimal_setter(self, value: bool) -> None:
        """Set arrow_number_to_decimal field. Deprecated.

        Allows setting this field through `cursor.connection.arrow_number_to_decimal_setter = True`.
        Added only because of backwards compatibility, correct setter should be used.
        """
        self.arrow_number_to_decimal = value

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

    @cached_property
    def snowflake_version(self) -> str:
        """The current Snowflake server version string."""
        with self.cursor(DictCursor) as cur:
            cur.execute("SELECT CURRENT_VERSION() AS version")
            row: dict[str, Any] = cur.fetchone()  # type: ignore[assignment]
        return str(row["VERSION"]).split(" ")[0]

    def get_query_status(self, sf_qid: str) -> QueryStatus:
        """Retrieve the status of query with sf_qid."""
        status, _ = self._get_query_status_with_response(sf_qid)
        return status

    def get_query_status_throw_if_error(self, sf_qid: str) -> QueryStatus:
        """Retrieve the status of query with sf_qid and raises an exception if the query terminated with an error."""
        status, response = self._get_query_status_with_response(sf_qid)
        if self.is_an_error(status):
            message = response.error_message if response.HasField("error_message") else f"Query {sf_qid} failed"
            errno = response.error_code if response.HasField("error_code") else -1
            raise ProgrammingError(msg=message, errno=errno, sfqid=sf_qid)
        return status

    def _get_query_status_with_response(self, sf_qid: str) -> tuple[QueryStatus, ConnectionGetQueryStatusResponse]:
        """Fetch query status from the server and map the status name to a QueryStatus enum value."""
        response = self.db_api.connection_get_query_status(
            ConnectionGetQueryStatusRequest(conn_handle=self.conn_handle, query_id=sf_qid)
        )
        try:
            status = QueryStatus[response.status_name]
        except KeyError:
            logger.warning("Unknown query status %r; treating as NO_DATA", response.status_name)
            status = QueryStatus.NO_DATA
        return status, response

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


# Backward compatibility alias
SnowflakeConnection = Connection
