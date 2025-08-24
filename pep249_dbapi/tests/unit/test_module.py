"""
Tests for PEP 249 module interface.
"""

import pytest

import pep249_dbapi

from pep249_dbapi import (
    apilevel, threadsafety, paramstyle, connect,
    Connection, Cursor, Warning, Error, InterfaceError, DatabaseError,
    DataError, OperationalError, IntegrityError, InternalError,
    ProgrammingError, NotSupportedError, Date, Time, Timestamp,
    DateFromTicks, TimeFromTicks, TimestampFromTicks, Binary,
    STRING, BINARY, NUMBER, DATETIME, ROWID
)


class TestModuleConstants:
    """Test module-level constants required by PEP 249."""
    
    def test_apilevel_constant(self):
        """Test apilevel constant."""
        assert apilevel == "2.0"
        assert hasattr(pep249_dbapi, 'apilevel')
        assert pep249_dbapi.apilevel == "2.0"
    
    def test_threadsafety_constant(self):
        """Test threadsafety constant."""
        assert threadsafety == 1
        assert hasattr(pep249_dbapi, 'threadsafety')
        assert pep249_dbapi.threadsafety == 1
        assert isinstance(threadsafety, int)
        assert 0 <= threadsafety <= 3  # Valid range according to PEP 249
    
    def test_paramstyle_constant(self):
        """Test paramstyle constant."""
        assert paramstyle == "format"
        assert hasattr(pep249_dbapi, 'paramstyle')
        assert pep249_dbapi.paramstyle == "format"
        
        # Valid paramstyle values according to PEP 249
        valid_paramstyles = ["format", "pyformat", "numeric", "named", "qmark"]
        assert paramstyle in valid_paramstyles


class TestModuleConnectFunction:
    """Test module-level connect function."""
    
    def test_connect_function_exists(self):
        """Test that connect function exists and is callable."""
        assert hasattr(pep249_dbapi, 'connect')
        assert callable(pep249_dbapi.connect)
        assert callable(connect)

    def test_connect_returns_connection(self, connection):
        """Test that connect returns a Connection object."""
        assert isinstance(connection, Connection)

class TestModuleExports:
    """Test that all required symbols are exported."""
    
    def test_connection_class_exported(self):
        """Test that Connection class is exported."""
        assert hasattr(pep249_dbapi, 'Connection')
        assert pep249_dbapi.Connection is Connection
    
    def test_cursor_class_exported(self):
        """Test that Cursor class is exported."""
        assert hasattr(pep249_dbapi, 'Cursor')
        assert pep249_dbapi.Cursor is Cursor
    
    def test_exception_classes_exported(self):
        """Test that all exception classes are exported."""
        exceptions = [
            Warning, Error, InterfaceError, DatabaseError, DataError,
            OperationalError, IntegrityError, InternalError,
            ProgrammingError, NotSupportedError
        ]
        
        for exc_class in exceptions:
            assert hasattr(pep249_dbapi, exc_class.__name__)
            assert getattr(pep249_dbapi, exc_class.__name__) is exc_class
    
    def test_type_constructors_exported(self):
        """Test that type constructor functions are exported."""
        constructors = [
            Date, Time, Timestamp, DateFromTicks, TimeFromTicks,
            TimestampFromTicks, Binary
        ]
        
        for constructor in constructors:
            assert hasattr(pep249_dbapi, constructor.__name__)
            assert getattr(pep249_dbapi, constructor.__name__) is constructor
    
    def test_type_objects_exported(self):
        """Test that type objects are exported."""
        type_object_names = ['STRING', 'BINARY', 'NUMBER', 'DATETIME', 'ROWID']
        
        for name in type_object_names:
            assert hasattr(pep249_dbapi, name), f"Type object '{name}' not exported"
    
    def test_all_exports_defined(self):
        """Test that __all__ is properly defined."""
        assert hasattr(pep249_dbapi, '__all__')
        assert isinstance(pep249_dbapi.__all__, list)
        
        # Check that all items in __all__ are actually available
        for name in pep249_dbapi.__all__:
            assert hasattr(pep249_dbapi, name), f"Symbol '{name}' in __all__ but not available"


class TestModuleDocumentation:
    """Test module documentation."""
    
    def test_module_has_docstring(self):
        """Test that module has a docstring."""
        assert pep249_dbapi.__doc__ is not None
        assert len(pep249_dbapi.__doc__.strip()) > 0
        assert "PEP 249" in pep249_dbapi.__doc__


class TestPEP249Compliance:
    """Test compliance with PEP 249 requirements."""
    
    def test_required_module_attributes(self):
        """Test that all required module attributes are present."""
        required_attrs = [
            'apilevel', 'threadsafety', 'paramstyle', 'connect',
            'Warning', 'Error', 'InterfaceError', 'DatabaseError',
            'DataError', 'OperationalError', 'IntegrityError',
            'InternalError', 'ProgrammingError', 'NotSupportedError',
            'Date', 'Time', 'Timestamp', 'DateFromTicks',
            'TimeFromTicks', 'TimestampFromTicks', 'Binary',
            'STRING', 'BINARY', 'NUMBER', 'DATETIME', 'ROWID'
        ]
        
        for attr in required_attrs:
            assert hasattr(pep249_dbapi, attr), f"Required attribute '{attr}' missing"
    
    def test_exception_hierarchy_compliance(self):
        """Test that exception hierarchy follows PEP 249."""
        # Warning should inherit from Warning (built-in)
        assert issubclass(Warning, Warning)
        
        # Error should be base of all database exceptions
        database_exceptions = [
            InterfaceError, DatabaseError, DataError, OperationalError,
            IntegrityError, InternalError, ProgrammingError, NotSupportedError
        ]
        
        for exc in database_exceptions:
            assert issubclass(exc, Error), f"{exc.__name__} should inherit from Error"
        
        # DatabaseError should be base of specific database errors
        database_specific_exceptions = [
            DataError, OperationalError, IntegrityError,
            InternalError, ProgrammingError, NotSupportedError
        ]
        
        for exc in database_specific_exceptions:
            assert issubclass(exc, DatabaseError), f"{exc.__name__} should inherit from DatabaseError"
    
    def test_type_constructors_callable(self):
        """Test that all type constructors are callable."""
        constructors = [
            Date, Time, Timestamp, DateFromTicks, TimeFromTicks,
            TimestampFromTicks, Binary
        ]
        
        for constructor in constructors:
            assert callable(constructor), f"{constructor.__name__} should be callable"
    
    def test_connection_interface_compliance(self):
        """Test that Connection class has required methods."""
        required_methods = [
            'close', 'commit', 'rollback', 'cursor'
        ]
        
        for method in required_methods:
            assert hasattr(Connection, method), f"Connection missing required method '{method}'"
            assert callable(getattr(Connection, method))
    
    def test_cursor_interface_compliance(self):
        """Test that Cursor class has required methods."""
        required_methods = [
            'callproc', 'close', 'execute', 'executemany',
            'fetchone', 'fetchmany', 'fetchall', 'nextset',
            'setinputsizes', 'setoutputsize'
        ]
        
        for method in required_methods:
            assert hasattr(Cursor, method), f"Cursor missing required method '{method}'"
            assert callable(getattr(Cursor, method))
        
        # Test required attributes
        required_attrs = ['description', 'rowcount', 'arraysize']
        
        for attr in required_attrs:
            assert hasattr(Cursor, attr), f"Cursor missing required attribute '{attr}'" 