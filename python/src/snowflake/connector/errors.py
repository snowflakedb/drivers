"""
PEP 249 Database API 2.0 Exception Classes

This module defines the exception hierarchy for the Snowflake connector.

The **active** exceptions (raised at runtime) are the PEP 249 hierarchy plus a
handful of driver-specific types (``MissingDependencyError``, config errors,
``ReauthenticationRequest``). These are what ``sf_core`` status codes map to via
``STATUS_TO_EXCEPTION``.

Everything after the "Backward compatibility" section below exists solely so
that ``from snowflake.connector.errors import BadGatewayError`` (etc.) does not
break user code written against the old ``snowflake-connector-python`` driver.
None of these classes are raised by the universal core at runtime:

  - **HTTP exceptions** (``BadRequest``, ``ServiceUnavailableError``, ...):
    In the old driver, Python's ``requests`` library returned HTTP status codes
    that were wrapped into typed exceptions and used as internal retry-loop
    control flow signals.  They leaked to users only when retries were
    exhausted.  In the universal core, the Rust core handles HTTP retries
    internally; by the time an error reaches Python it is already mapped to a
    PEP 249 type via ``StatusCode``.

  - **Auth / token exceptions** (``RefreshTokenError``, ``TokenExpiredError``):
    Used in the old driver as internal signals between the OKTA authenticator
    and the retry loop. The universal core handles token refresh in Rust.

  - **TLS exception** (``RevocationCheckError``):
    OCSP/CRL verification runs inside the Rust TLS layer.

  - **File-transfer exceptions** (``BindUploadError``, ``RequestExceedMaxRetryError``,
    ``PresignedUrlExpiredError``):
    Stage upload/download retry logic is internal to the Rust core.

All backward-compatibility classes are marked with ``@backward_compatibility``.
"""

from __future__ import annotations

import warnings

from typing import TYPE_CHECKING, Any

from ._internal.backward_compatibility import install_backward_compatibility_getattr
from ._internal.decorators import backward_compatibility


if TYPE_CHECKING:
    from .aio.connection import Connection as AsyncConnection
    from .aio.cursor import SnowflakeCursorBase as AsyncSnowflakeCursorBase
    from .connection import Connection
    from .cursor import SnowflakeCursorBase

ErrorValue = dict[str, Any]

# ---------------------------------------------------------------------------
# PEP 249 exception hierarchy (active — raised at runtime)
# ---------------------------------------------------------------------------


class Warning(Warning):  # type: ignore[misc]
    """Exception raised for important warnings like data truncations while inserting, etc."""

    pass


class Error(Exception):
    """Exception that is the base class of all other error exceptions.

    In addition to being a standard PEP 249 exception base, this class provides
    the **error-handler protocol** used throughout the driver to raise errors in
    a way that is consistent with PEP 249 and backward-compatible with the old
    ``snowflake-connector-python`` driver.
    """

    def __init__(
        self,
        msg: str = "",
        errno: int = -1,
        sqlstate: str | None = None,
        sfqid: str | None = None,
        query: str | None = None,
        request_id: str | None = None,
        parameter: str | None = None,
        validation_code: int | None = None,
        **kwargs: Any,  # absorbs extra keys for backward compatibility with old driver code
    ) -> None:
        self.errno = errno
        self.sqlstate = sqlstate
        self.sfqid = sfqid
        self.query = query
        # Client-generated ``requestId`` sent on the query submission request.
        # Populated for errors that originate from a query execution attempt;
        # ``None`` for non-query errors (auth, config, transport).
        self.request_id = request_id
        self.raw_msg = msg
        self.msg = self._format_message(msg)
        # The offending config parameter and structured sf_core ValidationCode
        # (see protobuf_gen's ValidationCode enum), when the error originated
        # from a connection-config validation failure. None otherwise.
        self.parameter = parameter
        self.validation_code = validation_code
        super().__init__(self.msg)

    def __repr__(self) -> str:
        return self.__str__()

    def __str__(self) -> str:
        return self.msg

    def _format_message(self, msg: str) -> str:
        code_str = f"{self.errno:06d}" if isinstance(self.errno, int) and self.errno >= 0 else "------"
        sqlstate_str = f" ({self.sqlstate})" if self.sqlstate else ""
        base = f"{code_str}{sqlstate_str}: {msg}" if msg else ""
        if not base:
            return base
        ids = []
        if self.request_id:
            ids.append(f"request_id={self.request_id}")
        if self.sfqid:
            ids.append(f"sfqid={self.sfqid}")
        return f"{base} ({', '.join(ids)})" if ids else base

    # ------------------------------------------------------------------
    # Error-handler protocol (PEP 249 / backward compatible)
    # ------------------------------------------------------------------

    @staticmethod
    def errorhandler_wrapper(
        connection: Connection | None,
        cursor: SnowflakeCursorBase | AsyncSnowflakeCursorBase | None,
        error_class: type[Error] | type[Exception],
        error_value: ErrorValue,
    ) -> None:
        """Raise an error through the error-handler chain.

        This is the **single entry point** that all driver code should use to
        report errors originating from connection or cursor operations.

        The method first tries to hand the error to a user-supplied handler via
        :pyfunc:`hand_to_other_handler`.  If no handler was available (both
        *connection* and *cursor* are ``None``), it falls back to creating and
        raising the exception directly.
        """
        handed_over = Error.hand_to_other_handler(connection, cursor, error_class, error_value)
        if not handed_over:
            raise Error.errorhandler_make_exception(error_class, error_value)

    @staticmethod
    def hand_to_other_handler(
        connection: Connection | AsyncConnection | None,
        cursor: SnowflakeCursorBase | AsyncSnowflakeCursorBase | None,
        error_class: type[Error] | type[Exception],
        error_value: ErrorValue,
    ) -> bool:
        """Try to delegate the error to a connection's or cursor's error handler.

        Records the error in the ``messages`` list of both the connection and
        the cursor (when present), then invokes the first available handler.

        Returns:
            ``True`` if a handler was invoked, ``False`` if both *connection*
            and *cursor* were ``None``.
        """
        if connection is not None:
            connection.messages.append((error_class, error_value))
        if cursor is not None:
            cursor.messages.append((error_class, error_value))
            cursor.errorhandler(connection, cursor, error_class, error_value)
            return True
        elif connection is not None:
            connection.errorhandler(connection, cursor, error_class, error_value)
            return True
        return False

    @staticmethod
    def errorhandler_make_exception(
        error_class: type[Error] | type[Exception],
        error_value: ErrorValue,
    ) -> Error | Exception:
        """Create an exception instance from *error_class* and *error_value*.

        Used as a fallback when no connection/cursor handler is available.
        """
        if issubclass(error_class, Error):
            return error_class(
                msg=error_value.get("msg", ""),
                errno=error_value.get("errno", -1),
                sqlstate=error_value.get("sqlstate"),
                sfqid=error_value.get("sfqid"),
                query=error_value.get("query"),
                request_id=error_value.get("request_id"),
                parameter=error_value.get("parameter"),
                validation_code=error_value.get("validation_code"),
            )
        return error_class(error_value)

    @staticmethod
    def default_errorhandler(
        connection: Connection | None,
        cursor: SnowflakeCursorBase | AsyncSnowflakeCursorBase | None,
        error_class: type[Error] | type[Exception],
        error_value: ErrorValue,
    ) -> None:
        """Default error handler that simply raises the error.

        This is the handler installed on every new ``Connection`` and ``Cursor``
        unless the user overrides ``errorhandler``.
        """
        # circular-import: errorhandler.py imports Error from this module at top
        # level, so this helper is imported at call time to avoid the cycle.
        from ._internal.errorhandler import _mark_errorhandler_routed

        new_exc = Error.errorhandler_make_exception(error_class, error_value)
        # The reconstructed exception must carry the routed flag so a
        # @simplified_error_handling caller does not PEP route twice
        _mark_errorhandler_routed(new_exc)
        raise new_exc


class InterfaceError(Error):
    """
    Exception raised for errors that are related to the database interface
    rather than the database itself.
    """

    pass


class DatabaseError(Error):
    """Exception raised for errors that are related to the database."""

    pass


class DataError(DatabaseError):
    """
    Exception raised for errors that are due to problems with the processed data
    like division by zero, numeric value out of range, etc.
    """

    pass


class OperationalError(DatabaseError):
    """
    Exception raised for errors that are related to the database's operation
    and not necessarily under the control of the programmer.
    """

    #: HTTP status code, set when this error represents a raw HTTP failure
    #: surfaced by ``SnowflakeRestful.fetch(..., raise_raw_http_failure=True)``.
    #: ``None`` for operational errors that are not HTTP failures.
    http_status: int | None = None

    @classmethod
    def from_http_response(cls, status_code: int, response_body: bytes) -> OperationalError:
        """Build an error representing a raw HTTP failure (status >= 400).

        The HTTP status is attached as :attr:`http_status` so consumers can
        branch on it without parsing the message string. (The old driver raised
        a vendored ``requests`` ``HTTPError`` carrying ``.response.status_code``;
        this is the universal-driver equivalent.)
        """
        # Truncate body to avoid flooding error messages with large payloads.
        error = cls(msg=f"HTTP {status_code}: {response_body[:200]!r}")
        error.http_status = status_code
        return error


class IntegrityError(DatabaseError):
    """
    Exception raised when the relational integrity of the database is affected,
    e.g. a foreign key check fails.
    """

    pass


class InternalError(DatabaseError):
    """Exception raised when the database encounters an internal error."""

    pass


class ProgrammingError(DatabaseError):
    """
    Exception raised for programming errors, e.g. table not found or already exists,
    syntax error in the SQL statement, wrong number of parameters specified, etc.
    """

    pass


class NotSupportedError(DatabaseError):
    """
    Exception raised in case a method or database API was used which is not
    supported by the database.
    """

    pass


# ---------------------------------------------------------------------------
# Driver-specific exceptions (active — raised at runtime)
# ---------------------------------------------------------------------------

_FUTURE_BASE_CHANGE_WARNED = False


def _warn_future_base_change() -> None:
    """Advise callers that ``ReauthenticationRequest``'s base will change.

    An advisory must never shadow the failure it is advising about: wrapped in
    try/except so a caller's ``filterwarnings("error")`` can't turn this into
    the exception the caller receives instead of ``ReauthenticationRequest``.
    Deduped once per process so a retry loop doesn't flood stderr.
    """
    global _FUTURE_BASE_CHANGE_WARNED
    if _FUTURE_BASE_CHANGE_WARNED:
        return
    _FUTURE_BASE_CHANGE_WARNED = True
    try:
        warnings.warn(
            "`ReauthenticationRequest` currently subclasses `ProgrammingError` for "
            "compatibility with `snowflake-connector-python`. In a future major "
            "release its base will change to `OperationalError`. If you catch this "
            "via `except ProgrammingError`, switch to `except ReauthenticationRequest` "
            "(or `except DatabaseError`) to stay correct across that change.",
            FutureWarning,
            stacklevel=3,
        )
    except Exception:
        pass


# NOTE: `ProgrammingError` is deliberately NOT the PEP 249-correct base here.
# `ud-py-pep249-error-subclass-selection` classes token expiry as environmental
# (`OperationalError`), and PEP 249's `OperationalError` example #1 is "an
# unexpected disconnect occurs". We subclass `ProgrammingError` anyway so that
# `except ProgrammingError` — what legacy snowflake-connector-python raised on
# the common mid-session path — keeps working for migrating callers. The base
# changes to `OperationalError` in a future major release: SNOW-3965765.
class ReauthenticationRequest(ProgrammingError):
    """Raised when the session cannot be renewed; open a new connection.

    Subclasses `ProgrammingError` only for compatibility with
    `snowflake-connector-python`; the base becomes `OperationalError` in a
    future major release. Catch this class directly, or `DatabaseError`, to be
    immune to that change.
    """

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        _warn_future_base_change()


class MissingDependencyError(Error):
    """Exception for missing extras dependencies."""

    def __init__(self, dependency: str) -> None:
        super().__init__(msg=f"Missing optional dependency: {dependency}")


class ConfigManagerError(Error):
    """Exception raised for configuration manager errors."""

    pass


class ConfigSourceError(Error):
    """Exception raised when a configuration source has invalid values."""

    pass


class MissingConfigOptionError(ConfigSourceError):
    """Exception raised when a required configuration option is missing."""

    pass


# ---------------------------------------------------------------------------
# Backward compatibility (importable, never raised by the universal core)
#
# See module docstring for rationale.
# ---------------------------------------------------------------------------


@backward_compatibility
class HttpError(Error):
    """Old-driver general HTTP exception."""


@backward_compatibility
class BadRequest(Error):
    """Old-driver exception for HTTP 400."""


@backward_compatibility
class ForbiddenError(Error):
    """Old-driver exception for HTTP 403."""


@backward_compatibility
class MethodNotAllowed(Error):
    """Old-driver exception for HTTP 405."""


@backward_compatibility
class RequestTimeoutError(Error):
    """Old-driver exception for HTTP 408."""


@backward_compatibility
class TooManyRequests(Error):
    """Old-driver exception for HTTP 429."""


@backward_compatibility
class InternalServerError(Error):
    """Old-driver exception for HTTP 500."""


@backward_compatibility
class BadGatewayError(Error):
    """Old-driver exception for HTTP 502."""


@backward_compatibility
class ServiceUnavailableError(Error):
    """Old-driver exception for HTTP 503."""


@backward_compatibility
class GatewayTimeoutError(Error):
    """Old-driver exception for HTTP 504."""


@backward_compatibility
class OtherHTTPRetryableError(Error):
    """Old-driver exception for unclassified retryable HTTP errors."""


@backward_compatibility
class RefreshTokenError(Error):
    """Old-driver internal signal for OAuth token refresh."""


@backward_compatibility
class TokenExpiredError(Error):
    """Old-driver internal signal for expired session tokens."""


@backward_compatibility
class RevocationCheckError(OperationalError):
    """Old-driver exception for OCSP/CRL revocation check failures."""


@backward_compatibility
class BindUploadError(Error):
    """Old-driver exception for stage upload failures during array binding."""


@backward_compatibility
class RequestExceedMaxRetryError(Error):
    """Old-driver exception for cloud storage REST calls exceeding max retries."""


@backward_compatibility
class PresignedUrlExpiredError(Error):
    """Old-driver exception for expired cloud storage presigned URLs."""


# Must be the last statement; see ``install_backward_compatibility_getattr``.
install_backward_compatibility_getattr(__name__)
