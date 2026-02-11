"""
PEP 249 Database API 2.0 Exception Classes

This module defines the exception hierarchy as specified in PEP 249.
"""

from __future__ import annotations

from snowflake.connector._internal.errorcode import ER_FAILED_TO_CONNECT_TO_DB, ER_INVALID_VALUE


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


# Configuration-related errors (for ConfigManager)


class ConfigManagerError(Error):
    """Exception raised for configuration manager errors."""

    pass


class ConfigSourceError(ConfigManagerError):
    """Exception raised when a configuration source has invalid values."""

    pass


class MissingConfigOptionError(ConfigSourceError):
    """Exception raised when a required configuration option is missing."""

    pass


###### BACK-COMPAT  ######


class BadRequest(Error):
    """Exception for 400 HTTP error for retry."""


class ForbiddenError(Error):
    """Exception for 403 HTTP error for retry."""


class BadGatewayError(Error):
    """Exception for 502 HTTP error for retry."""


# ---------------------------------------------------------------------------
# Proto status-code → PEP 249 exception class mapping
# ---------------------------------------------------------------------------

# Internal proto StatusCode enum values (from database_driver_v1.proto)
_STATUS_CODE_GENERIC_ERROR = 1
_STATUS_CODE_AUTHENTICATION_ERROR = 2
_STATUS_CODE_INVALID_ARGUMENT = 3
_STATUS_CODE_TIMEOUT = 4
_STATUS_CODE_NOT_FOUND = 5
_STATUS_CODE_ALREADY_EXISTS = 6
_STATUS_CODE_NOT_IMPLEMENTED = 7
_STATUS_CODE_UNAUTHORIZED = 8
_STATUS_CODE_CANCELLED = 9
_STATUS_CODE_INVALID_STATE = 10
_STATUS_CODE_RESOURCE_EXHAUSTED = 11
_STATUS_CODE_ADBC_INTERNAL = 12
_STATUS_CODE_IO = 13
_STATUS_CODE_INTERNAL_ERROR = 14
_STATUS_CODE_MISSING_PARAMETER = 15
_STATUS_CODE_INVALID_PARAMETER_VALUE = 16
_STATUS_CODE_LOGIN_ERROR = 17
_STATUS_CODE_INVALID_DATA = 18

_STATUS_TO_EXCEPTION: dict[int, type[Error]] = {
    _STATUS_CODE_GENERIC_ERROR: DatabaseError,
    _STATUS_CODE_AUTHENTICATION_ERROR: DatabaseError,
    _STATUS_CODE_INVALID_ARGUMENT: ProgrammingError,
    _STATUS_CODE_TIMEOUT: OperationalError,
    _STATUS_CODE_NOT_FOUND: ProgrammingError,
    _STATUS_CODE_ALREADY_EXISTS: ProgrammingError,
    _STATUS_CODE_NOT_IMPLEMENTED: NotSupportedError,
    _STATUS_CODE_UNAUTHORIZED: OperationalError,
    _STATUS_CODE_CANCELLED: OperationalError,
    _STATUS_CODE_INVALID_STATE: InternalError,
    _STATUS_CODE_RESOURCE_EXHAUSTED: OperationalError,
    _STATUS_CODE_ADBC_INTERNAL: InternalError,
    _STATUS_CODE_IO: OperationalError,
    _STATUS_CODE_INTERNAL_ERROR: ProgrammingError,
    _STATUS_CODE_MISSING_PARAMETER: ProgrammingError,
    _STATUS_CODE_INVALID_PARAMETER_VALUE: ProgrammingError,
    _STATUS_CODE_LOGIN_ERROR: DatabaseError,
    _STATUS_CODE_INVALID_DATA: DataError,
}

# Human-readable label for each status code (used as fallback message)
_STATUS_CODE_LABELS: dict[int, str] = {
    _STATUS_CODE_GENERIC_ERROR: "Generic error",
    _STATUS_CODE_AUTHENTICATION_ERROR: "Authentication error",
    _STATUS_CODE_INVALID_ARGUMENT: "Invalid argument",
    _STATUS_CODE_TIMEOUT: "Timeout",
    _STATUS_CODE_NOT_FOUND: "Not found",
    _STATUS_CODE_ALREADY_EXISTS: "Already exists",
    _STATUS_CODE_NOT_IMPLEMENTED: "Not implemented",
    _STATUS_CODE_UNAUTHORIZED: "Unauthorized",
    _STATUS_CODE_CANCELLED: "Cancelled",
    _STATUS_CODE_INVALID_STATE: "Invalid state",
    _STATUS_CODE_RESOURCE_EXHAUSTED: "Resource exhausted",
    _STATUS_CODE_ADBC_INTERNAL: "ADBC internal error",
    _STATUS_CODE_IO: "I/O error",
    _STATUS_CODE_INTERNAL_ERROR: "Internal error",
    _STATUS_CODE_MISSING_PARAMETER: "Missing parameter",
    _STATUS_CODE_INVALID_PARAMETER_VALUE: "Invalid parameter value",
    _STATUS_CODE_LOGIN_ERROR: "Login error",
    _STATUS_CODE_INVALID_DATA: "Invalid data",
}

# Map proto StatusCode → errno aligned with the reference snowflake-connector-python.
# For query errors (INTERNAL_ERROR) the Snowflake server code is exposed via the
# proto vendor_code field and takes precedence in the conversion layer.
_STATUS_TO_ERRNO: dict[int, int] = {
    _STATUS_CODE_AUTHENTICATION_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    _STATUS_CODE_LOGIN_ERROR: ER_FAILED_TO_CONNECT_TO_DB,
    _STATUS_CODE_INVALID_PARAMETER_VALUE: ER_INVALID_VALUE,
}
