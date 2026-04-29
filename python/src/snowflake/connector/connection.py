"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

import atexit
import functools
import logging
import platform
import re
import threading
import warnings

from collections.abc import Generator, Iterable
from functools import cached_property
from io import StringIO
from typing import Any, Callable, TypeVar, Union, cast

from snowflake.connector._internal.config_utils import create_config_settings_from_dict, pop_typed_kwarg
from snowflake.connector._internal.errorcode import ER_CONNECTION_IS_CLOSED, ER_INVALID_VALUE
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
    WrapperIdentity,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionCloseRequest,
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionGetParameterRequest,
    ConnectionGetQueryStatusRequest,
    ConnectionGetQueryStatusResponse,
    ConnectionHeartbeatRequest,
    ConnectionInitRequest,
    ConnectionIsClosedRequest,
    ConnectionNewRequest,
    ConnectionReleaseRequest,
    ConnectionSetOptionsRequest,
    ConnectionSetSessionParametersRequest,
    DatabaseInitRequest,
    DatabaseNewRequest,
    DatabaseReleaseRequest,
)
from snowflake.connector._internal.sqlstate import SQLSTATE_CONNECTION_NOT_EXISTS

from ._internal._private_key_helper import normalize_private_key
from ._internal.api_client.client_api import database_driver_client
from ._internal.binding_converters import ParamStyle
from ._internal.decorators import backward_compatibility, internal_api, pep249
from ._internal.errorhandler import ErrorHandlerMixin
from ._internal.extras import check_dependency
from ._internal.extras import numpy as np
from ._internal.logout_config_mapping import (
    LogoutConfig,
    LogoutOptionKeys,
)
from ._internal.snowflake_restful import SnowflakeRestful
from ._internal.text_utils import split_statements
from .constants import QueryStatus
from .cursor import CursorInstance, CursorType, DictCursor, SnowflakeCursor
from .errors import DatabaseError, Error, ErrorValue, InterfaceError, ProgrammingError
from .telemetry import TelemetryClient
from .version import __version__


# backward compatibility constant
# snowflake-sqlalchemy imports this symbol and calls .get(name) in
# parse_query_param_type to cast URL query-string values to the types the
# connector expects.  The universal driver validates parameters internally, so
# an empty dict is correct: every .get() returns None and values pass through
# uncast.
DEFAULT_CONFIGURATION: dict[str, tuple[Any, tuple[type, ...]]] = {}

CLIENT_NAME = "snowflake-connector-python"
_APPLICATION_NAME = "PythonConnector"
# The old connector used re.match(r"[\w\d_]+") without anchors, so any string
# starting with a word character was accepted (dots, hyphens, etc. in the tail
# were silently ignored).  We keep a start-anchored pattern without $ so that
# callers like Snow CLI can pass dotted names such as "SNOWCLI.STAGE.COPY".
APPLICATION_RE = re.compile(r"^[\w\d_]+")
LOG_MAX_QUERY_LENGTH = 80

SessionParameters = dict[str, Any]
ConnectionParamValue = Union[int, str, float, bytes, bool, SessionParameters]
ConnectionParameters = dict[str, ConnectionParamValue]

# Module-level logger
logger = logging.getLogger(__name__)


F = TypeVar("F", bound=Callable[..., Any])


def _requires_open(func: F) -> F:
    """Raise ``DatabaseError`` if the connection is closed (mirrors old driver behavior)."""
    # TODO: it should rather raise InterfaceError, consider to align with the cursor

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
            server_session_keep_alive: Optional[bool] - Control server session lifecycle
                - True: Never send logout (Fire & Forget; session persists on server
                  as long as there is activity in it, e.g. running queries)
                - False: Always send logout on close. For backward compatibility,
                  False is currently remapped to None when auto-detection is enabled,
                  so Core checks the async query registry before logout.
                  This remapping will be removed in a future version.
                - None: Delegate to auto-detection setting
            enable_server_session_keep_alive_auto_detection: Optional[bool]
                - True (default): Check async query registry before logout (backward compat)
                - False: Don't check registry
                - None: Auto-detection disabled (Core treats None as False)
            auto_cleanup: bool - Enable atexit handler for automatic connection cleanup
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
        self._messages: list[tuple[type[Exception], ErrorValue]] = []
        self._errorhandler: Callable[..., None] = Error.default_errorhandler

        # paramstyle (via setter so str | ParamStyle normalization is single-sourced)
        from snowflake.connector import paramstyle as default_paramstyle

        self.paramstyle = paramstyle or default_paramstyle

        kwargs = self._rewrite_private_key_password(kwargs)
        kwargs = self._rewrite_mfa_params(kwargs)

        self._log_max_query_length: int = kwargs.get("log_max_query_length", LOG_MAX_QUERY_LENGTH)  # type: ignore[assignment]

        application = kwargs.pop("application", None)
        if application is None or (isinstance(application, str) and not application):
            self._application = _APPLICATION_NAME
        elif isinstance(application, str):
            if not APPLICATION_RE.match(application):
                raise ProgrammingError(msg=f"Invalid application name: {application!r}")
            self._application = application
        else:
            raise ProgrammingError(msg=f"Invalid application parameter (must be a non-empty string): {application!r}")
        kwargs["client_app_id"] = self._application

        # Extract Python-only params before processing kwargs for Rust core
        self._numpy: bool = self._resolve_numpy_option(kwargs)
        self._arrow_number_to_decimal: bool = bool(kwargs.pop("arrow_number_to_decimal", False))

        self.db_api = database_driver_client()
        self.db_handle: DatabaseHandle | None = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle: ConnectionHandle | None = self.db_api.connection_new(ConnectionNewRequest()).conn_handle

        if autocommit is not None and not isinstance(autocommit, bool):
            raise ProgrammingError(f"Invalid autocommit parameter: {autocommit!r}")

        # Pop all special-purpose keys from kwargs in-place.
        # After this, kwargs contains only generic Core options.
        self._parse_kwargs(kwargs, autocommit)

        # All driver config → Core in a single RPC (generic kwargs + logout config)
        self._send_driver_options(kwargs)

        # Session params → Core (separate RPC: these are Snowflake server
        # SET commands (string→string), not driver config (typed ConfigSetting))
        if self._session_params:
            self.db_api.connection_set_session_parameters(
                ConnectionSetSessionParametersRequest(conn_handle=self.conn_handle, parameters=self._session_params)
            )

        self._connect()

        _sensitive_keys = {"password", "private_key", "passcode", "private_key_password", "private_key_file_pwd"}
        self.kwargs = {k: ("***" if k in _sensitive_keys else v) for k, v in kwargs.items()}
        self._close_lock = threading.Lock()

    def _connect(self) -> None:
        """Establish the connection to Snowflake via the Rust core."""
        self.db_api.connection_init(
            ConnectionInitRequest(
                conn_handle=self.conn_handle,
                db_handle=self.db_handle,
                wrapper_identity=WrapperIdentity(
                    driver_name=CLIENT_NAME,
                    driver_version=__version__,
                    language_runtime=platform.python_implementation(),
                    language_version=platform.python_version(),
                    language_compiler=platform.python_compiler(),
                ),
            )
        )

        if self._should_auto_cleanup():
            atexit.register(self._close_at_process_exit)

    def _parse_kwargs(self, kwargs: dict[str, Any], autocommit: bool | None) -> None:
        """Parse and extract all special params from kwargs in-place.

        After this call, kwargs contains only generic Core options
        suitable for connection_set_options. Special params are stored
        on self (auto_cleanup, _session_params, _numpy, logout_config).
        """
        # Python-only (pop — never goes to Core)
        self.auto_cleanup: bool = pop_typed_kwarg(kwargs, "auto_cleanup", bool, True)

        # Session params use a dedicated RPC (connection_set_session_parameters),
        # not the generic connection_set_options path, so pop them from kwargs.
        self._session_params = self._extract_session_params(kwargs, autocommit)

        # Transform in-place (stays in kwargs for generic path)
        if "private_key" in kwargs:
            kwargs["private_key"] = normalize_private_key(kwargs["private_key"])

        # Logout params (pop + resolve defaults + build config).
        # Init-time snapshot only; Core re-derives at close() time from connection_seed,
        # so post-init overrides like close(retry=False) won't be reflected here.
        self.logout_config = LogoutConfig.from_kwargs(kwargs)

    @staticmethod
    def _extract_session_params(kwargs: dict[str, Any], autocommit: bool | None) -> SessionParameters:
        """Pop session_parameters from kwargs and fold in autocommit."""
        params: SessionParameters = kwargs.pop("session_parameters", None) or {}
        if autocommit is not None:
            params["AUTOCOMMIT"] = str(autocommit).lower()
        return params

    def _send_driver_options(self, kwargs: dict[str, Any]) -> None:
        """Send all driver config to Core in a single connection_set_options RPC.

        Combines generic kwargs (user, account, host, ...) with resolved logout
        config from self.logout_config (server_session_keep_alive, error_strategy,
        timeouts, ...) into one batched call. Called at init time, before connection_init.
        """
        options = create_config_settings_from_dict(kwargs)
        options.update(create_config_settings_from_dict(self.logout_config.to_option_dict()))
        if options:
            response = self.db_api.connection_set_options(
                ConnectionSetOptionsRequest(
                    conn_handle=self.conn_handle,
                    options=options,
                )
            )
            for warning in response.warnings:
                warnings.warn(warning.message, stacklevel=2)

    @pep249
    def close(self, retry: bool = True) -> None:
        """
        Close the connection, send logout, and release handles.

        Args:
            retry: If False, overrides max_attempts to 1 (no retries) before closing.
                   If True (default), uses init-time configuration.

        Thread-safety: the lock guards only the handle swap (nanoseconds, no I/O).
        All FFI calls use local handle copies outside the lock, so concurrent
        close() calls are safe — the second caller gets None handles and skips.
        """
        atexit.unregister(self._close_at_process_exit)

        # Fast path — Core query, no lock. Core marks is_closed=true via AtomicBool
        # at the START of connection_close (before HTTP logout), so this returns True
        # even while another thread's logout is still in-flight.
        if self.is_closed():
            return

        # Lock guards ONLY the handle swap — prevents concurrent double-release.
        with self._close_lock:
            del self._messages[:]
            conn_handle, self.conn_handle = self.conn_handle, None
            db_handle, self.db_handle = self.db_handle, None

        # All I/O outside the lock, using local handle copies.
        # try/finally ensures handles are always released — on success, Strict
        # failure, or set_options failure.
        try:
            if conn_handle:
                if not retry:
                    self.db_api.connection_set_options(
                        ConnectionSetOptionsRequest(
                            conn_handle=conn_handle,
                            options=create_config_settings_from_dict({LogoutOptionKeys.LOGOUT_MAX_ATTEMPTS: 1}),
                        )
                    )

                # Logout + mark closed in Core (network I/O, bounded by Core's timeout)
                self.db_api.connection_close(ConnectionCloseRequest(conn_handle=conn_handle))
        finally:
            # Release handles in Core's object store.
            # Separated from connection_close for future connection pooling.
            if conn_handle:
                self._release_connection_handle(conn_handle)
            if db_handle:
                self._release_database_handle(db_handle)

    def _try_close(self) -> None:
        """Best-effort close for __del__ and atexit — never raises."""
        try:
            if not self.is_closed():
                self.close(retry=False)
        except Exception:
            try:
                logger.debug("close() failed during cleanup")
            except Exception:
                pass

    def _should_auto_cleanup(self) -> bool:
        """Whether this connection should auto-close on GC/exit.

        Uses getattr with False as fallback (NOT the default value of auto_cleanup,
        which is True). False here is a GC fail-safe: if __del__ fires on a
        half-initialized object (exception during __init__), we must NOT attempt
        cleanup on an object whose Core handles were never created.
        """
        return getattr(self, "auto_cleanup", False)

    def __del__(self) -> None:
        if self._should_auto_cleanup():
            self._try_close()

    def _release_connection_handle(self, conn_handle: ConnectionHandle) -> None:
        """Release the Rust-side connection handle."""
        try:
            self.db_api.connection_release(ConnectionReleaseRequest(conn_handle=conn_handle))
        except Exception:
            logger.warning("Failed to release connection handle", exc_info=True)

    def _release_database_handle(self, db_handle: DatabaseHandle) -> None:
        """Release the Rust-side database handle."""
        try:
            self.db_api.database_release(DatabaseReleaseRequest(db_handle=db_handle))
        except Exception:
            logger.warning("Failed to release database handle", exc_info=True)

    def _close_at_process_exit(self) -> None:
        """
        Cleanup handler called by atexit when process exits.

        If close() was called successfully, this handler should have been unregistered
        and should NOT run. If it runs for an already-closed connection, that indicates
        a potential bug (unregister failed, race condition, or multiple registrations).

        The entire body is wrapped in try/except because during interpreter shutdown,
        any call (FFI, logging, warnings) may fail due to torn-down module state.
        """
        try:
            if self.is_closed():
                logger.debug(
                    "atexit handler ran for already-closed connection. "
                    "This may indicate atexit.unregister() failed or a race condition occurred."
                )
                return

            # Connection is leaked (not explicitly closed) — emit FutureWarning.
            # Auto-cleanup will be disabled by default in a future version (SNOW-2314152).
            try:
                warnings.warn(
                    "Connection was not explicitly closed before process exit. "
                    "Auto-cleanup at exit will be disabled by default in a future version. "
                    "Please explicitly call connection.close() or use context manager.",
                    FutureWarning,
                    stacklevel=2,
                )
            except Exception:
                pass  # Interpreter shutting down; warning emission is best-effort

            self._try_close()
        except Exception:
            try:
                logger.warning("_close_at_process_exit failed during interpreter shutdown")
            except Exception:
                pass  # logger itself may be torn down

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        """List of (exception class, exception value) tuples received from the database."""
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    @pep249
    @_requires_open
    def commit(self) -> None:
        """Commit any pending transaction to the database."""
        cur = self.cursor()
        try:
            cur.execute("COMMIT")
        finally:
            cur.close()

    @pep249
    @_requires_open
    def rollback(self) -> None:
        """Roll back to the start of any pending transaction."""
        cur = self.cursor()
        try:
            cur.execute("ROLLBACK")
        finally:
            cur.close()

    @pep249
    @_requires_open
    def cursor(self, cursor_class: CursorType = SnowflakeCursor) -> CursorInstance:
        """
        Return a new Cursor object using the connection.

        Args:
            cursor_class: The class to use for the cursor (default: SnowflakeCursor).
                          Pass DictCursor to get results as dictionaries.

        Returns:
            SnowflakeCursorBase: A new cursor object
        """
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
        """Exit the runtime context. Commit on success / rollback on exception if autocommit is OFF."""
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
        value = self._get_session_parameter("AUTOCOMMIT")
        return value is not None and value.lower() == "true"

    @_requires_open
    def set_autocommit(self, autocommit: bool) -> None:
        """Set the autocommit mode. Executes ALTER SESSION SET autocommit on the server."""
        # FIXME: set autocommit via core
        if not isinstance(autocommit, bool):
            raise ProgrammingError(msg=f"Invalid autocommit parameter: {autocommit!r}", errno=ER_INVALID_VALUE)
        cur = self.cursor()
        try:
            cur.execute(f"ALTER SESSION SET autocommit={str(autocommit).lower()}")
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

        Queries Core's authoritative state. If the handle has been released
        (connection_release after close), the query fails — treated as closed
        since a released handle means close() already completed.
        """
        try:
            response = self.db_api.connection_is_closed(ConnectionIsClosedRequest(conn_handle=self.conn_handle))
            return bool(response.is_closed)
        except Exception:
            # Handle released or FFI unavailable — connection is closed
            return True

    def is_valid(self) -> bool:
        """Check whether the connection is still usable for sending queries.

        Validates both the network transport and the Snowflake session by sending a heartbeat to the server.
        """
        if self.is_closed():
            return False
        try:
            request = ConnectionHeartbeatRequest(conn_handle=self.conn_handle)
            response = self.db_api.connection_heartbeat(request)
            return response.valid
        except Exception:
            return False

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
    def _get_connection_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        """Refresh connection details for connection"""
        return self.db_api.connection_get_info(
            ConnectionGetInfoRequest(
                conn_handle=self.conn_handle,
                include_master_token=include_master_token,
            )
        )

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
            raise InterfaceError(msg="Session ID is not available; connection may not be initialized")
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
        # Bare raise: we need a working errorhandler to route errors through the protocol.
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

    @property
    def _errorhandler_connection(self) -> Connection:
        return self

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
        return self._log_max_query_length

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
        return self._arrow_number_to_decimal

    @arrow_number_to_decimal.setter
    def arrow_number_to_decimal(self, value: bool) -> None:
        self._arrow_number_to_decimal = bool(value)

    @arrow_number_to_decimal.setter  # type: ignore[attr-defined, untyped-decorator]
    @backward_compatibility
    def arrow_number_to_decimal_setter(self, value: bool) -> None:
        """Set arrow_number_to_decimal field. Deprecated.

        Kept so legacy code that writes
        ``cursor.connection.arrow_number_to_decimal_setter = True`` keeps
        working; new code should assign to ``arrow_number_to_decimal``
        directly.
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
        if self.is_closed():
            return QueryStatus.DISCONNECTED, ConnectionGetQueryStatusResponse()
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
    def _resolve_numpy_option(kwargs: ConnectionParameters) -> bool:
        """Pop ``numpy`` from *kwargs* and validate that numpy is installed if requested."""
        use_numpy = bool(kwargs.pop("numpy", False))
        if use_numpy:
            check_dependency(np)
        return use_numpy

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
