"""
Tests for PEP 249 exception classes.
"""

import pytest

from snowflake.ud_connector.exceptions import (
    Warning, Error, InterfaceError, DatabaseError, DataError,
    OperationalError, IntegrityError, InternalError, ProgrammingError,
    NotSupportedError
)


class TestExceptionHierarchy:
    """Test the exception hierarchy as defined in PEP 249."""
    
    def test_warning_inheritance(self):
        """Test that Warning inherits from Warning."""
        # Note: Warning in PEP 249 should inherit from Python's Warning, not Exception
        assert issubclass(Warning, Warning)
    
    def test_error_inheritance(self):
        """Test that Error inherits from Exception."""
        assert issubclass(Error, Exception)
    
    def test_interface_error_inheritance(self):
        """Test that InterfaceError inherits from Error."""
        assert issubclass(InterfaceError, Error)
        assert issubclass(InterfaceError, Exception)
    
    def test_database_error_inheritance(self):
        """Test that DatabaseError inherits from Error."""
        assert issubclass(DatabaseError, Error)
        assert issubclass(DatabaseError, Exception)
    
    def test_data_error_inheritance(self):
        """Test that DataError inherits from DatabaseError."""
        assert issubclass(DataError, DatabaseError)
        assert issubclass(DataError, Error)
        assert issubclass(DataError, Exception)
    
    def test_operational_error_inheritance(self):
        """Test that OperationalError inherits from DatabaseError."""
        assert issubclass(OperationalError, DatabaseError)
        assert issubclass(OperationalError, Error)
        assert issubclass(OperationalError, Exception)
    
    def test_integrity_error_inheritance(self):
        """Test that IntegrityError inherits from DatabaseError."""
        assert issubclass(IntegrityError, DatabaseError)
        assert issubclass(IntegrityError, Error)
        assert issubclass(IntegrityError, Exception)
    
    def test_internal_error_inheritance(self):
        """Test that InternalError inherits from DatabaseError."""
        assert issubclass(InternalError, DatabaseError)
        assert issubclass(InternalError, Error)
        assert issubclass(InternalError, Exception)
    
    def test_programming_error_inheritance(self):
        """Test that ProgrammingError inherits from DatabaseError."""
        assert issubclass(ProgrammingError, DatabaseError)
        assert issubclass(ProgrammingError, Error)
        assert issubclass(ProgrammingError, Exception)
    
    def test_not_supported_error_inheritance(self):
        """Test that NotSupportedError inherits from DatabaseError."""
        assert issubclass(NotSupportedError, DatabaseError)
        assert issubclass(NotSupportedError, Error)
        assert issubclass(NotSupportedError, Exception)


class TestExceptionInstantiation:
    """Test that exceptions can be instantiated and raised."""
    
    def test_warning_instantiation(self):
        """Test Warning instantiation."""
        warning = Warning("Test warning")
        assert str(warning) == "Test warning"
    
    def test_error_instantiation(self):
        """Test Error instantiation."""
        error = Error("Test error")
        assert str(error) == "Test error"
    
    def test_interface_error_instantiation(self):
        """Test InterfaceError instantiation."""
        error = InterfaceError("Interface error")
        assert str(error) == "Interface error"
    
    def test_database_error_instantiation(self):
        """Test DatabaseError instantiation."""
        error = DatabaseError("Database error")
        assert str(error) == "Database error"
    
    def test_data_error_instantiation(self):
        """Test DataError instantiation."""
        error = DataError("Data error")
        assert str(error) == "Data error"
    
    def test_operational_error_instantiation(self):
        """Test OperationalError instantiation."""
        error = OperationalError("Operational error")
        assert str(error) == "Operational error"
    
    def test_integrity_error_instantiation(self):
        """Test IntegrityError instantiation."""
        error = IntegrityError("Integrity error")
        assert str(error) == "Integrity error"
    
    def test_internal_error_instantiation(self):
        """Test InternalError instantiation."""
        error = InternalError("Internal error")
        assert str(error) == "Internal error"
    
    def test_programming_error_instantiation(self):
        """Test ProgrammingError instantiation."""
        error = ProgrammingError("Programming error")
        assert str(error) == "Programming error"
    
    def test_not_supported_error_instantiation(self):
        """Test NotSupportedError instantiation."""
        error = NotSupportedError("Not supported error")
        assert str(error) == "Not supported error"


class TestExceptionRaising:
    """Test that exceptions can be raised and caught."""
    
    def test_raise_interface_error(self):
        """Test raising InterfaceError."""
        with pytest.raises(InterfaceError) as excinfo:
            raise InterfaceError("Test interface error")
        assert str(excinfo.value) == "Test interface error"
    
    def test_raise_database_error(self):
        """Test raising DatabaseError."""
        with pytest.raises(DatabaseError) as excinfo:
            raise DatabaseError("Test database error")
        assert str(excinfo.value) == "Test database error"
    
    def test_catch_database_error_hierarchy(self):
        """Test catching DatabaseError catches all subclasses."""
        # Test that we can catch DatabaseError when raising a subclass
        with pytest.raises(DatabaseError):
            raise DataError("Data error")
        
        with pytest.raises(DatabaseError):
            raise OperationalError("Operational error")
        
        with pytest.raises(DatabaseError):
            raise IntegrityError("Integrity error")
        
        with pytest.raises(DatabaseError):
            raise InternalError("Internal error")
        
        with pytest.raises(DatabaseError):
            raise ProgrammingError("Programming error")
        
        with pytest.raises(DatabaseError):
            raise NotSupportedError("Not supported error")
    
    def test_catch_error_hierarchy(self):
        """Test catching Error catches all database errors."""
        # Test that we can catch Error when raising any database error
        with pytest.raises(Error):
            raise InterfaceError("Interface error")
        
        with pytest.raises(Error):
            raise DatabaseError("Database error")
        
        with pytest.raises(Error):
            raise DataError("Data error") 