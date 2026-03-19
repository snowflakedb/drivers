"""Error Handling E2E tests for Universal Driver.

This module tests error handling functionality including:
- SQL syntax errors with sqlstate
- Non-existent table errors
- Non-existent database errors
- DROP IF EXISTS handling
- Closed cursor errors
- Error attribute population
- Exception hierarchy validation
- Cursor recovery after error
"""

from __future__ import annotations

import pytest

from snowflake.connector import DatabaseError, Error, InterfaceError, ProgrammingError
from tests.e2e.types.utils import assert_connection_is_open


class TestErrorHandling:
    """Tests for error handling with structured error information."""

    def test_should_return_structured_error_for_sql_syntax_error(self, execute_query, cursor):
        """Test that SQL syntax error raises ProgrammingError with sqlstate."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Invalid SQL "SELCT INVALID SYNTAX" is executed
        with pytest.raises(ProgrammingError) as exc_info:
            cursor.execute("SELCT INVALID SYNTAX")

        # Then A programming error should be raised with sqlstate "42000"
        error = exc_info.value
        assert error.sqlstate == "42000"

        # And The error should have a non-empty errno
        assert isinstance(error.errno, int)
        assert error.errno > 0

        # And The error should have sqlstate "42000"
        assert error.sqlstate == "42000"

        # And The error should have a non-empty message
        assert isinstance(error.msg, str)
        assert len(error.msg) > 0

    def test_should_return_error_for_non_existent_table(self, execute_query, cursor):
        """Test error for non-existent table."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When "SELECT * FROM this_table_does_not_exist_e2e_xyz" is executed
        with pytest.raises(ProgrammingError) as exc_info:
            cursor.execute("SELECT * FROM this_table_does_not_exist_e2e_xyz")

        # Then An error should be raised with errno 2003
        error = exc_info.value
        assert error.errno == 2003

        # And The error message should contain the table name
        assert "this_table_does_not_exist_e2e_xyz".upper() in error.msg.upper()

    def test_should_return_error_for_non_existent_database(self, execute_query, cursor):
        """Test error for non-existent database."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When "USE DATABASE this_db_does_not_exist_e2e_xyz" is executed
        with pytest.raises(ProgrammingError):
            cursor.execute("USE DATABASE this_db_does_not_exist_e2e_xyz")

        # Then An error should be raised
        assert True  # Verified by pytest.raises above

    def test_should_succeed_silently_for_drop_if_exists_on_non_existent_table(self, execute_query, cursor):
        """Test that DROP IF EXISTS succeeds silently for non-existent table."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When "DROP TABLE IF EXISTS this_table_does_not_exist_e2e_xyz" is executed
        cursor.execute("DROP TABLE IF EXISTS this_table_does_not_exist_e2e_xyz")
        result = cursor.fetchone()

        # Then No error should be raised
        assert result is not None

    def test_should_raise_interface_error_on_operations_on_closed_cursor(self, execute_query, connection):
        """Test that operations on closed cursor raise InterfaceError."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A cursor is created and used and then closed
        cursor = connection.cursor()
        cursor.execute("SELECT 1")
        cursor.fetchone()
        cursor.close()

        # When execute() is called on the closed cursor
        with pytest.raises(InterfaceError) as exc_info:
            cursor.execute("SELECT 1")

        # Then InterfaceError should be raised
        assert exc_info.type is InterfaceError

    def test_should_maintain_correct_exception_hierarchy(self):
        """Test that exception hierarchy is maintained correctly."""
        # Given The snowflake.connector error module is imported
        assert ProgrammingError is not None

        # When The exception classes are inspected
        assert all(cls is not None for cls in [ProgrammingError, DatabaseError, Error, InterfaceError])

        # Then ProgrammingError should be a subclass of DatabaseError
        assert issubclass(ProgrammingError, DatabaseError)

        # And DatabaseError should be a subclass of Error
        assert issubclass(DatabaseError, Error)

        # And InterfaceError should be a subclass of Error
        assert issubclass(InterfaceError, Error)

        # And InterfaceError should NOT be a subclass of DatabaseError
        assert not issubclass(InterfaceError, DatabaseError)
