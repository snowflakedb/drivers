"""
PEP 249 Database API 2.0 Exception Classes

This module defines the exception hierarchy as specified in PEP 249.
"""

from __future__ import annotations

from snowflake.connector._internal.errorcode import ER_HTTP_GENERAL_ERROR


class Warning(Warning):  # type: ignore[misc]
    """Exception raised for important warnings like data truncations while inserting, etc."""

    pass


class Error(Exception):
    """Exception that is the base class of all other error exceptions."""

    def __init__(
        self,
        msg: str = "",
        errno: int = -1,
        sqlstate: str | None = None,
        sfqid: str | None = None,
        query: str | None = None,
    ) -> None:
        self.errno = errno
        self.sqlstate = sqlstate
        self.sfqid = sfqid
        self.query = query
        self.raw_msg = msg
        self.msg = self._format_message(msg)
        super().__init__(self.msg)

    def _format_message(self, msg: str) -> str:
        code_str = f"{self.errno:06d}" if isinstance(self.errno, int) and self.errno >= 0 else "------"
        sqlstate_str = f" ({self.sqlstate})" if self.sqlstate else ""
        return f"{code_str}{sqlstate_str}: {msg}" if msg else ""


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

    pass


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


class MissingDependencyError(Error):
    """Exception for missing extras dependencies."""

    def __init__(self, dependency: str) -> None:
        super().__init__(msg=f"Missing optional dependency: {dependency}")


# Configuration-related errors (for ConfigManager)


class ConfigManagerError(Error):
    """Exception raised for configuration manager errors."""

    pass


class ConfigSourceError(Error):
    """Exception raised when a configuration source has invalid values."""

    pass


class MissingConfigOptionError(ConfigSourceError):
    """Exception raised when a required configuration option is missing."""

    pass


# HTTP exceptions — all inherit Error directly, matching the reference driver.


def _http_init(self: Error, default_msg: str, **kwargs: object) -> None:
    Error.__init__(
        self,
        msg=kwargs.get("msg") or default_msg,  # type: ignore[arg-type]
        errno=ER_HTTP_GENERAL_ERROR + int(kwargs.get("errno", 0) or 0),
        sqlstate=kwargs.get("sqlstate"),  # type: ignore[arg-type]
        sfqid=kwargs.get("sfqid"),  # type: ignore[arg-type]
    )


class BadRequest(Error):
    """Exception for 400 HTTP error for retry."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 400: Bad Request", **kwargs)


class ForbiddenError(Error):
    """Exception for 403 HTTP error for retry."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 403: Forbidden", **kwargs)


class MethodNotAllowed(Error):
    """Exception for HTTP 405 Method Not Allowed."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 405: Method Not Allowed", **kwargs)


class RequestTimeoutError(Error):
    """Exception for HTTP 408 Request Timeout."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 408: Request Timeout", **kwargs)


class TooManyRequests(Error):
    """Exception for HTTP 429 Too Many Requests."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 429: Too Many Requests", **kwargs)


class InternalServerError(Error):
    """Exception for HTTP 500 Internal Server Error."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 500: Internal Server Error", **kwargs)


class BadGatewayError(Error):
    """Exception for HTTP 502 Bad Gateway."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 502: Bad Gateway", **kwargs)


class ServiceUnavailableError(Error):
    """Exception for HTTP 503 Service Unavailable."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 503: Service Unavailable", **kwargs)


class GatewayTimeoutError(Error):
    """Exception for HTTP 504 Gateway Timeout."""

    def __init__(self, **kwargs: object) -> None:
        _http_init(self, "HTTP 504: Gateway Timeout", **kwargs)


class OtherHTTPRetryableError(Error):
    """Exception for other HTTP error for retry."""

    def __init__(self, **kwargs: object) -> None:
        code = kwargs.get("code", "n/a")
        _http_init(self, f"HTTP {code}", **kwargs)


# Auth / token exceptions


class RefreshTokenError(Error):
    """Exception raised when an OAuth token refresh fails."""

    def __init__(self, **kwargs: object) -> None:
        Error.__init__(
            self,
            msg=kwargs.get("msg") or "Token Refresh Required",  # type: ignore[arg-type]
            errno=kwargs.get("errno"),  # type: ignore[arg-type]
            sqlstate=kwargs.get("sqlstate"),  # type: ignore[arg-type]
            sfqid=kwargs.get("sfqid"),  # type: ignore[arg-type]
        )


class TokenExpiredError(Error):
    """Exception raised when a session token has expired and cannot be refreshed."""

    pass


# TLS / certificate exceptions


class RevocationCheckError(OperationalError):
    """Exception raised when a certificate revocation check (OCSP/CRL) fails."""

    pass


# File transfer exceptions


class BindUploadError(Error):
    """Exception raised when a stage upload for array binding fails."""

    pass


class RequestExceedMaxRetryError(Error):
    """Exception raised when cloud storage REST calls exceed the maximum retry count."""

    pass


class PresignedUrlExpiredError(Error):
    """Exception raised when a cloud storage presigned URL has expired."""

    pass
