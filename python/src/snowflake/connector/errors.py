"""
PEP 249 Database API 2.0 Exception Classes

This module defines the exception hierarchy as specified in PEP 249.
"""

from __future__ import annotations


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
