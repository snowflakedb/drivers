"""
PEP 249 Database API 2.0 Exception Classes

This module defines the exception hierarchy as specified in PEP 249.
"""


class Warning(Warning):
    """
    Exception raised for important warnings like data truncations while inserting, etc.
    """
    pass


class Error(Exception):
    """
    Exception that is the base class of all other error exceptions.
    """
    pass


class InterfaceError(Error):
    """
    Exception raised for errors that are related to the database interface
    rather than the database itself.
    """
    pass


class DatabaseError(Error):
    """
    Exception raised for errors that are related to the database.
    """
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
    """
    Exception raised when the database encounters an internal error.
    """
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