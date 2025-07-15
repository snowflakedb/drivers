"""
PEP 249 Database API 2.0 Type Objects and Constructors

This module defines the type constructors and type objects as specified in PEP 249.
"""

import datetime
import time


# Type Constructors
def Date(year, month, day):
    """
    Construct an object holding a date value.
    
    Args:
        year: Year
        month: Month
        day: Day
        
    Returns:
        datetime.date: Date object
    """
    return datetime.date(year, month, day)


def Time(hour, minute, second):
    """
    Construct an object holding a time value.
    
    Args:
        hour: Hour
        minute: Minute
        second: Second
        
    Returns:
        datetime.time: Time object
    """
    return datetime.time(hour, minute, second)


def Timestamp(year, month, day, hour, minute, second):
    """
    Construct an object holding a timestamp value.
    
    Args:
        year: Year
        month: Month
        day: Day
        hour: Hour
        minute: Minute
        second: Second
        
    Returns:
        datetime.datetime: Timestamp object
    """
    return datetime.datetime(year, month, day, hour, minute, second)


def DateFromTicks(ticks):
    """
    Construct an object holding a date value from the given ticks value.
    
    Args:
        ticks: Seconds since the epoch
        
    Returns:
        datetime.date: Date object
    """
    return datetime.date.fromtimestamp(ticks)


def TimeFromTicks(ticks):
    """
    Construct an object holding a time value from the given ticks value.
    
    Args:
        ticks: Seconds since the epoch
        
    Returns:
        datetime.time: Time object
    """
    return datetime.datetime.fromtimestamp(ticks).time()


def TimestampFromTicks(ticks):
    """
    Construct an object holding a timestamp value from the given ticks value.
    
    Args:
        ticks: Seconds since the epoch
        
    Returns:
        datetime.datetime: Timestamp object
    """
    return datetime.datetime.fromtimestamp(ticks)


def Binary(string):
    """
    Construct an object capable of holding a binary (long) string value.
    
    Args:
        string: Binary data
        
    Returns:
        bytes: Binary object
    """
    if isinstance(string, str):
        return string.encode('utf-8')
    return bytes(string)


# Type Objects for comparison
class DBAPITypeObject:
    """
    Base class for type objects that support comparison with database types.
    """
    def __init__(self, *values):
        self.values = values
    
    def __eq__(self, other):
        return other in self.values
    
    def __ne__(self, other):
        return other not in self.values


# Type objects for describing database column types
STRING = DBAPITypeObject("STRING", "VARCHAR", "CHAR", "TEXT")
BINARY = DBAPITypeObject("BINARY", "VARBINARY", "BLOB")
NUMBER = DBAPITypeObject("NUMBER", "INTEGER", "INT", "FLOAT", "DOUBLE", "DECIMAL", "NUMERIC")
DATETIME = DBAPITypeObject("DATETIME", "DATE", "TIME", "TIMESTAMP")
ROWID = DBAPITypeObject("ROWID", "OID") 