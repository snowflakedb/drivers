"""
Integration tests for error handling.

Tests verify that real Snowflake errors are surfaced as proper PEP 249 exceptions
with meaningful messages and structured attributes.

These tests are designed to pass against both the new (universal) driver and
the old (reference) snowflake-connector-python driver.
"""

import uuid

import pytest

from snowflake.connector.errors import DatabaseError, Error, InterfaceError, ProgrammingError
from tests.compatibility import is_new_driver, is_old_driver


# Password authenticator name differs between drivers.
# Old driver: "snowflake" (DEFAULT_AUTHENTICATOR = "SNOWFLAKE")
# New driver: "SNOWFLAKE_PASSWORD"
PASSWORD_AUTH = "SNOWFLAKE_PASSWORD" if is_new_driver() else "snowflake"


class TestAuthenticationErrors:
    """Test that authentication failures raise proper errors."""

    def test_invalid_password(self, connection_factory):
        """Test that wrong password raises a DatabaseError subclass with descriptive message."""
        with pytest.raises(DatabaseError) as excinfo:
            connection_factory(authenticator=PASSWORD_AUTH, password="wrong_password_12345")
        error = excinfo.value
        assert "incorrect username or password" in error.msg.lower()
        assert error.errno == 250001

    def test_invalid_user(self, connection_factory):
        """Test that a non-existent user raises a DatabaseError subclass."""
        with pytest.raises(DatabaseError) as excinfo:
            connection_factory(
                authenticator=PASSWORD_AUTH,
                user=f"nonexistent_user_{uuid.uuid4().hex[:8]}",
                password="dummy",
            )
        error = excinfo.value
        assert error.msg
        assert error.errno == 250001

    def test_invalid_account(self, connection_factory):
        """Test that a non-existent account raises an Error subclass."""
        with pytest.raises(Error) as excinfo:
            connection_factory(
                account=f"nonexistent_account_{uuid.uuid4().hex[:8]}",
                authenticator=PASSWORD_AUTH,
                password="dummy",
            )
        error = excinfo.value
        assert error.errno != -1
        if is_new_driver():
            assert error.errno == 250001
        else:
            # Old driver: HttpError (290404) when server returns 404 for unknown account,
            # or ER_FAILED_TO_CONNECT_TO_DB (250001) for connection-level failures.
            assert error.errno in (290404, 250001)

    def test_invalid_authenticator_value(self, connection_factory):
        """Test that an unsupported authenticator value raises ProgrammingError."""
        with pytest.raises(ProgrammingError) as excinfo:
            connection_factory(authenticator="INVALID_AUTH_METHOD", password="dummy")
        error = excinfo.value
        assert "authenticator" in error.msg.lower()
        assert error.errno == 251007


class TestQuerySyntaxErrors:
    """Test that SQL syntax errors raise DatabaseError with descriptive messages."""

    def test_invalid_sql_syntax(self, cursor):
        """Test that malformed SQL raises DatabaseError mentioning the syntax error."""
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELEC 1")
        error = excinfo.value
        assert error.errno == 1003
        assert "sql compilation error" in error.msg.lower()
        assert "syntax error" in error.msg.lower()

    def test_unclosed_string_literal(self, cursor):
        """Test that an unclosed string literal raises DatabaseError with parse error."""
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELECT 'unclosed")
        error = excinfo.value
        assert error.errno == 1003
        assert "sql compilation error" in error.msg.lower()
        assert "parse error" in error.msg.lower()

    def test_invalid_identifier(self, cursor):
        """Test that referencing a non-existent column raises DatabaseError."""
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELECT nonexistent_column")
        error = excinfo.value
        assert error.errno == 904
        assert "sql compilation error" in error.msg.lower()
        assert "invalid identifier" in error.msg.lower()


class TestObjectNotFoundErrors:
    """Test that references to non-existent objects raise DatabaseError."""

    def test_select_from_nonexistent_table(self, cursor):
        """Test that selecting from a non-existent table raises DatabaseError."""
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name}")
        error = excinfo.value
        assert error.errno == 2003
        assert "does not exist or not authorized" in error.msg.lower()

    def test_drop_nonexistent_table(self, cursor):
        """Test that dropping a non-existent table (without IF EXISTS) raises DatabaseError."""
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"DROP TABLE {table_name}")
        error = excinfo.value
        assert error.errno == 2003
        assert "does not exist or not authorized" in error.msg.lower()

    def test_use_nonexistent_database(self, cursor):
        """Test that USE on a non-existent database raises DatabaseError."""
        db_name = f"nonexistent_db_{uuid.uuid4().hex[:8]}"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"USE DATABASE {db_name}")
        error = excinfo.value
        assert error.errno == 2043


class TestClosedCursorErrors:
    """Test that operations on a closed cursor raise proper errors."""

    def test_execute_on_closed_cursor(self, cursor):
        """Test that execute on a closed cursor raises InterfaceError."""
        cursor.close()
        with pytest.raises(InterfaceError, match="(?i)cursor is closed"):
            cursor.execute("SELECT 1")

    def test_fetchone_on_closed_cursor(self, cursor):
        """Test that fetchone on a closed cursor raises an error."""
        cursor.close()
        # New driver raises InterfaceError; old driver raises TypeError (no closed-cursor guard on fetch).
        with pytest.raises((InterfaceError, TypeError)):
            cursor.fetchone()

    def test_fetchall_on_closed_cursor(self, cursor):
        """Test that fetchall on a closed cursor raises an error."""
        cursor.close()
        # New driver raises InterfaceError; old driver raises TypeError (no closed-cursor guard on fetch).
        with pytest.raises((InterfaceError, TypeError)):
            cursor.fetchall()


class TestClosedConnectionErrors:
    """Test that operations on a closed connection raise proper errors."""

    def test_cursor_on_closed_connection(self, connection_factory):
        """Test that creating a cursor on a closed connection raises an error."""
        conn = connection_factory()
        conn.close()
        # New driver: InterfaceError; old driver: DatabaseError (errno=250002)
        with pytest.raises(Error, match="(?i)connection is closed"):
            conn.cursor()

    def test_execute_on_closed_connection(self, connection_factory):
        """Test that execute via cursor on a closed connection raises an error."""
        conn = connection_factory()
        cur = conn.cursor()
        conn.close()
        # New driver: InterfaceError("Connection is closed.")
        # Old driver: InterfaceError("Cursor is closed in execute.") or DatabaseError("Connection is closed")
        with pytest.raises(Error, match="(?i)(?:connection|cursor) is closed"):
            cur.execute("SELECT 1")


class TestErrorAttributes:
    """Test that errors raised from real queries carry expected PEP 249 attributes."""

    def test_error_inherits_from_database_error(self, cursor):
        """Test that a query error is catchable as DatabaseError and Error."""
        with pytest.raises(DatabaseError):
            cursor.execute("SELEC 1")

    def test_error_has_errno(self, cursor):
        """Test that errors from the server carry the Snowflake server error code."""
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        error = excinfo.value
        assert error.errno == 1003

    def test_error_has_raw_msg(self, cursor):
        """Test that errors from the server carry raw_msg."""
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        assert excinfo.value.raw_msg is not None
        assert "sql compilation error" in excinfo.value.raw_msg.lower()

    def test_error_has_sqlstate(self, cursor):
        """Test that errors from the server carry sqlstate."""
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        assert excinfo.value.sqlstate == "42000"

    def test_error_does_not_leak_internal_cause(self, cursor):
        """Test that server errors do not expose internal proto exceptions via __cause__."""
        if is_old_driver():
            pytest.skip("__cause__ suppression is a new-driver concern")
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        assert excinfo.value.__cause__ is None
