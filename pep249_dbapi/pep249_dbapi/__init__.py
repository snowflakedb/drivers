"""
PEP 249 Database API 2.0 Implementation

This module provides an empty implementation of the Python Database API Specification 2.0
as defined in PEP 249.
"""
from .api_client.c_api import register_default_logger_callback
from .connection import Connection
from .cursor import Cursor
from .exceptions import (
    Warning, Error, InterfaceError, DatabaseError, DataError, OperationalError,
    IntegrityError, InternalError, ProgrammingError, NotSupportedError
)
from .types import (
    Date, Time, Timestamp, DateFromTicks, TimeFromTicks, TimestampFromTicks,
    Binary, STRING, BINARY, NUMBER, DATETIME, ROWID
)

# Module Interface Constants
apilevel = "2.0"
threadsafety = 1  # Threads may share the module, but not connections
paramstyle = "format"  # Python extended format codes, e.g. ...WHERE name=%s

register_default_logger_callback()

def connect(**kwargs):
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
    # Module constants
    'apilevel', 'threadsafety', 'paramstyle',

    # Connection function
    'connect',

    # Classes
    'Connection', 'Cursor',

    # Exceptions
    'Warning', 'Error', 'InterfaceError', 'DatabaseError', 'DataError',
    'OperationalError', 'IntegrityError', 'InternalError', 'ProgrammingError',
    'NotSupportedError',

    # Type constructors
    'Date', 'Time', 'Timestamp', 'DateFromTicks', 'TimeFromTicks',
    'TimestampFromTicks', 'Binary',

    # Type objects
    'STRING', 'BINARY', 'NUMBER', 'DATETIME', 'ROWID'
]