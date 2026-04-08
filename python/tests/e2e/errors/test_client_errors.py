"""
E2E tests for client-side error scenarios.

Tests verify that client-side errors (closed connections, invalid parameters,
executemany issues, data conversion) raise proper PEP 249 exceptions.
"""

import uuid

import pytest

from snowflake.connector.errors import (
    DatabaseError,
    Error,
    InterfaceError,
    ProgrammingError,
)
from tests.compatibility import is_new_driver


PASSWORD_AUTH = "SNOWFLAKE_PASSWORD" if is_new_driver() else "snowflake"


class TestClosedConnectionCursorErrors:
    """Tests for errors when using closed connections or cursors."""

    def test_should_raise_database_error_when_creating_cursor_on_closed_connection(
        self, connection_factory
    ):
        # Given A connection that has been closed
        conn = connection_factory()
        conn.close()

        # When The user attempts to create a cursor
        # Then DatabaseError is raised with message matching "Connection is closed"
        with pytest.raises(Error, match="(?i)connection is closed"):
            conn.cursor()

    def test_should_raise_interface_error_when_executing_on_closed_cursor(self, cursor):
        # Given A cursor that has been closed
        cursor.close()

        # When The user calls execute with "SELECT 1"
        # Then InterfaceError is raised with message matching "Cursor is closed"
        with pytest.raises(InterfaceError, match="(?i)cursor is closed"):
            cursor.execute("SELECT 1")

    def test_should_raise_error_when_executing_on_cursor_after_connection_closed(
        self, connection_factory
    ):
        # Given A connection with an open cursor
        conn = connection_factory()
        cur = conn.cursor()

        # When The connection is closed
        conn.close()

        # And The user calls execute on the cursor
        # Then Error is raised with message matching "closed"
        with pytest.raises(Error, match="(?i)closed"):
            cur.execute("SELECT 1")


class TestInvalidConnectionParameterErrors:
    """Tests for errors with invalid connection parameters."""

    def test_should_raise_programming_error_for_invalid_authenticator_value(
        self, connection_factory
    ):
        # When The user connects with authenticator "INVALID_AUTH_METHOD"
        # Then ProgrammingError is raised with errno 251007
        with pytest.raises(ProgrammingError) as excinfo:
            connection_factory(authenticator="INVALID_AUTH_METHOD", password="dummy")
        assert excinfo.value.errno == 251007

    def test_should_raise_programming_error_for_malformed_private_key(
        self, connection_factory
    ):
        # When The user connects with SNOWFLAKE_JWT and invalid private_key bytes
        # Then ProgrammingError is raised with message matching "private key"
        with pytest.raises(ProgrammingError, match="(?i)private key"):
            connection_factory(
                authenticator="SNOWFLAKE_JWT",
                private_key=b"not-a-valid-private-key",
            )


class TestExecutemanyErrors:
    """Tests for executemany error scenarios."""

    def test_should_raise_interface_error_for_executemany_with_non_rewritable_insert(
        self, cursor
    ):
        # Given A temporary table with schema "val INT"
        table_name = f"test_executemany_err_{uuid.uuid4().hex[:8]}"
        cursor.execute(f"CREATE TEMPORARY TABLE {table_name} (val INT)")

        try:
            # When executemany is called with "INSERT INTO t (SELECT 1)" and [[1], [2]]
            # Then InterfaceError is raised with message matching "Failed to rewrite multi-row insert"
            with pytest.raises(InterfaceError, match="(?i)failed to rewrite"):
                cursor.executemany(
                    f"INSERT INTO {table_name} (SELECT 1)", [[1], [2]]
                )
        finally:
            cursor.execute(f"DROP TABLE IF EXISTS {table_name}")

    @pytest.mark.parametrize("connection", ["qmark"], indirect=True)
    def test_should_raise_interface_error_for_executemany_with_inconsistent_row_sizes(
        self, cursor
    ):
        # Given A temporary table with schema "val INT" and qmark paramstyle
        table_name = f"test_executemany_rows_{uuid.uuid4().hex[:8]}"
        cursor.execute(f"CREATE TEMPORARY TABLE {table_name} (val INT)")

        try:
            # When executemany is called with "INSERT INTO t VALUES (?)" and [[1], [1, 2]]
            # Then InterfaceError is raised with message matching "Bulk data size don't match"
            with pytest.raises(InterfaceError, match="(?i)bulk data size"):
                cursor.executemany(
                    f"INSERT INTO {table_name} VALUES (?)", [[1], [1, 2]]
                )
        finally:
            cursor.execute(f"DROP TABLE IF EXISTS {table_name}")


class TestDataConversionErrors:
    """Tests for data conversion error scenarios."""

    def test_should_raise_interface_error_for_timestamp_with_out_of_range_year(
        self, cursor
    ):
        # When The user executes "SELECT '12345-01-02'::TIMESTAMP_NTZ" and calls fetchone
        # Then InterfaceError is raised with message matching "Failed to convert"
        cursor.execute("SELECT '12345-01-02'::TIMESTAMP_NTZ")
        with pytest.raises(InterfaceError, match="(?i)failed to convert"):
            cursor.fetchone()
