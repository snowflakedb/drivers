"""
Tests for PEP 249 type objects and constructors.
"""

import datetime
import time
import pytest

from pep249_dbapi.types import (
    Date, Time, Timestamp, DateFromTicks, TimeFromTicks, TimestampFromTicks,
    Binary, STRING, BINARY, NUMBER, DATETIME, ROWID, DBAPITypeObject
)


class TestTypeConstructors:
    """Test type constructor functions."""
    
    def test_date_constructor(self):
        """Test Date constructor."""
        result = Date(2023, 12, 25)
        assert isinstance(result, datetime.date)
        assert result.year == 2023
        assert result.month == 12
        assert result.day == 25
    
    def test_time_constructor(self):
        """Test Time constructor."""
        result = Time(14, 30, 45)
        assert isinstance(result, datetime.time)
        assert result.hour == 14
        assert result.minute == 30
        assert result.second == 45
    
    def test_timestamp_constructor(self):
        """Test Timestamp constructor."""
        result = Timestamp(2023, 12, 25, 14, 30, 45)
        assert isinstance(result, datetime.datetime)
        assert result.year == 2023
        assert result.month == 12
        assert result.day == 25
        assert result.hour == 14
        assert result.minute == 30
        assert result.second == 45
    
    def test_date_from_ticks(self):
        """Test DateFromTicks constructor."""
        ticks = time.time()
        result = DateFromTicks(ticks)
        assert isinstance(result, datetime.date)
        # Should be approximately today's date
        expected = datetime.date.fromtimestamp(ticks)
        assert result == expected
    
    def test_time_from_ticks(self):
        """Test TimeFromTicks constructor."""
        ticks = time.time()
        result = TimeFromTicks(ticks)
        assert isinstance(result, datetime.time)
        # Should be approximately current time
        expected = datetime.datetime.fromtimestamp(ticks).time()
        assert result == expected
    
    def test_timestamp_from_ticks(self):
        """Test TimestampFromTicks constructor."""
        ticks = time.time()
        result = TimestampFromTicks(ticks)
        assert isinstance(result, datetime.datetime)
        # Should be approximately current timestamp
        expected = datetime.datetime.fromtimestamp(ticks)
        assert result == expected
    
    def test_binary_constructor_from_string(self):
        """Test Binary constructor with string input."""
        result = Binary("hello")
        assert isinstance(result, bytes)
        assert result == b"hello"
    
    def test_binary_constructor_from_bytes(self):
        """Test Binary constructor with bytes input."""
        input_bytes = b"\x00\x01\x02\x03"
        result = Binary(input_bytes)
        assert isinstance(result, bytes)
        assert result == input_bytes
    
    def test_binary_constructor_from_list(self):
        """Test Binary constructor with list input."""
        input_list = [65, 66, 67]  # ABC
        result = Binary(input_list)
        assert isinstance(result, bytes)
        assert result == b"ABC"


class TestTypeObjects:
    """Test type objects for database column type comparison."""
    
    def test_dbapi_type_object_creation(self):
        """Test DBAPITypeObject creation."""
        type_obj = DBAPITypeObject("TYPE1", "TYPE2")
        assert type_obj.values == ("TYPE1", "TYPE2")
    
    def test_dbapi_type_object_equality(self):
        """Test DBAPITypeObject equality comparison."""
        type_obj = DBAPITypeObject("TYPE1", "TYPE2")
        
        assert type_obj == "TYPE1"
        assert type_obj == "TYPE2"
        assert not (type_obj == "TYPE3")
    
    def test_dbapi_type_object_inequality(self):
        """Test DBAPITypeObject inequality comparison."""
        type_obj = DBAPITypeObject("TYPE1", "TYPE2")
        
        assert not (type_obj != "TYPE1")
        assert not (type_obj != "TYPE2")
        assert type_obj != "TYPE3"
    
    def test_string_type_object(self):
        """Test STRING type object."""
        assert STRING == "STRING"
        assert STRING == "VARCHAR"
        assert STRING == "CHAR"
        assert STRING == "TEXT"
        assert not (STRING == "INTEGER")
    
    def test_binary_type_object(self):
        """Test BINARY type object."""
        assert BINARY == "BINARY"
        assert BINARY == "VARBINARY"
        assert BINARY == "BLOB"
        assert not (BINARY == "TEXT")
    
    def test_number_type_object(self):
        """Test NUMBER type object."""
        assert NUMBER == "NUMBER"
        assert NUMBER == "INTEGER"
        assert NUMBER == "INT"
        assert NUMBER == "FLOAT"
        assert NUMBER == "DOUBLE"
        assert NUMBER == "DECIMAL"
        assert NUMBER == "NUMERIC"
        assert not (NUMBER == "TEXT")
    
    def test_datetime_type_object(self):
        """Test DATETIME type object."""
        assert DATETIME == "DATETIME"
        assert DATETIME == "DATE"
        assert DATETIME == "TIME"
        assert DATETIME == "TIMESTAMP"
        assert not (DATETIME == "INTEGER")
    
    def test_rowid_type_object(self):
        """Test ROWID type object."""
        assert ROWID == "ROWID"
        assert ROWID == "OID"
        assert not (ROWID == "INTEGER") 