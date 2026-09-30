"""
PEP 249 Database API 2.0 Implementation

This module provides an empty implementation of the Python Database API Specification 2.0
as defined in PEP 249.
"""

from __future__ import annotations

from importlib import import_module
from importlib.metadata import PackageNotFoundError, distribution
from typing import Any


# The distribution guard must run before importing any other connector modules:
# both wheels install this package, regardless of which one was installed last.
try:
    distribution("snowflake-connector-python-fips-candidate")
except PackageNotFoundError:
    pass
else:
    try:
        distribution("snowflake-connector-python")
    except PackageNotFoundError:
        pass
    else:
        raise RuntimeError(
            "snowflake-connector-python and snowflake-connector-python-fips-candidate "
            "cannot be installed together: uninstall both distributions, then install "
            "only the one you intend to use."
        )

    # Candidate wheels must carry a matching profile and native bridge. Ordinary
    # source installs can be pure Python, so do not require the bridge for them.
    from ._internal._distribution_profile import FIPS_TLS_CANDIDATE

    if not FIPS_TLS_CANDIDATE:
        raise RuntimeError(
            "The installed FIPS-TLS candidate metadata conflicts with the "
            "snowflake.connector FIPS-TLS build profile and native bridge. "
            "Uninstall both distributions, then reinstall only the intended wheel."
        )
    import_module("snowflake.connector._core")


from . import util_text  # noqa: F401 - backward compatibility re-exports
from ._internal.decorators import pep249
from .connection import Connection, SnowflakeConnection
from .connection_config import ConnectionConfig
from .constants import QueryStatus, StatementParameterName
from .cursor import DictCursor, SnowflakeCursor
from .errors import (
    DatabaseError,
    DataError,
    Error,
    IntegrityError,
    InterfaceError,
    InternalError,
    NotSupportedError,
    OperationalError,
    ProgrammingError,
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
threadsafety = 2  # Threads may share the module and connections, but not cursors
paramstyle = "pyformat"  # Default: %(name)s and %s placeholders (client-side interpolation)

# Sentinel to distinguish "not provided" from explicit values. Forwarding ``None``
# defaults would make ``@api_telemetry`` treat connection_name/config as passed.
_UNSET = object()


@pep249
def connect(
    *,
    connection_name: str | None | object = _UNSET,
    connections_file_path: str | None | object = _UNSET,
    config: ConnectionConfig | None | object = _UNSET,
    **kwargs: Any,
) -> Connection:
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
    conn_kwargs = dict(kwargs)
    if connection_name is not _UNSET:
        conn_kwargs["connection_name"] = connection_name
    if connections_file_path is not _UNSET:
        conn_kwargs["connections_file_path"] = connections_file_path
    if config is not _UNSET:
        conn_kwargs["config"] = config
    return Connection(**conn_kwargs)


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
    "ConnectionConfig",
    "Connection",
    "SnowflakeConnection",
    "QueryStatus",
    "StatementParameterName",
    "DictCursor",
    "SnowflakeCursor",
    # Exceptions
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
