"""
PEP 249 Database API 2.0 Implementation

This module provides an empty implementation of the Python Database API Specification 2.0
as defined in PEP 249.
"""

from typing import Any

from ._internal.api_client.c_api import register_default_logger_callback
from ._internal.decorators import pep249
from .connection import Connection, SnowflakeConnection
from .constants import QueryStatus
from .cursor import DictCursor, SnowflakeCursor
from .errors import (
    BadGatewayError,
    BadRequest,
    BindUploadError,
    ConfigManagerError,
    ConfigSourceError,
    DatabaseError,
    DataError,
    Error,
    ForbiddenError,
    GatewayTimeoutError,
    IntegrityError,
    InterfaceError,
    InternalError,
    InternalServerError,
    MethodNotAllowed,
    MissingConfigOptionError,
    MissingDependencyError,
    NotSupportedError,
    OperationalError,
    OtherHTTPRetryableError,
    PresignedUrlExpiredError,
    ProgrammingError,
    RefreshTokenError,
    RequestExceedMaxRetryError,
    RequestTimeoutError,
    RevocationCheckError,
    ServiceUnavailableError,
    TokenExpiredError,
    TooManyRequests,
    Warning,
)
from .types import (
    BINARY,
    DATETIME,
    NUMBER,
    ROWID,
    STRING,
    Binary,
    Date,
    DateFromTicks,
    Time,
    TimeFromTicks,
    Timestamp,
    TimestampFromTicks,
)
from .version import __version__


# PEP 249 Module Interface Constants
apilevel = "2.0"
threadsafety = 1  # Threads may share the module, but not connections
paramstyle = "pyformat"  # Default: %(name)s and %s placeholders (client-side interpolation)

register_default_logger_callback()


@pep249
def connect(**kwargs: Any) -> Connection:
    """
    Create a connection to the database.

    Args:
        database: Database name
        user: Username
        password: Password
        host: Host name
        port: Port number
        **kwargs: Additional connection parameters

    Returns:
        Connection: A Connection object
    """
    return Connection(**kwargs)


# Export all public symbols
__all__ = [
    # Version
    "__version__",
    # Module constants
    "apilevel",
    "threadsafety",
    "paramstyle",
    # Connection function
    "connect",
    # Classes
    "Connection",
    "SnowflakeConnection",
    "QueryStatus",
    "DictCursor",
    "SnowflakeCursor",
    # Exceptions — PEP 249 core
    "Warning",
    "Error",
    "InterfaceError",
    "DatabaseError",
    "DataError",
    "OperationalError",
    "IntegrityError",
    "InternalError",
    "ProgrammingError",
    "NotSupportedError",
    # Exceptions — configuration
    "ConfigManagerError",
    "ConfigSourceError",
    "MissingConfigOptionError",
    "MissingDependencyError",
    # Exceptions — HTTP
    "BadRequest",
    "ForbiddenError",
    "MethodNotAllowed",
    "RequestTimeoutError",
    "TooManyRequests",
    "InternalServerError",
    "BadGatewayError",
    "ServiceUnavailableError",
    "GatewayTimeoutError",
    "OtherHTTPRetryableError",
    # Exceptions — auth / token
    "RefreshTokenError",
    "TokenExpiredError",
    # Exceptions — TLS
    "RevocationCheckError",
    # Exceptions — file transfer
    "BindUploadError",
    "RequestExceedMaxRetryError",
    "PresignedUrlExpiredError",
    # Type constructors
    "Date",
    "Time",
    "Timestamp",
    "DateFromTicks",
    "TimeFromTicks",
    "TimestampFromTicks",
    "Binary",
    # Type objects
    "STRING",
    "BINARY",
    "NUMBER",
    "DATETIME",
    "ROWID",
]
