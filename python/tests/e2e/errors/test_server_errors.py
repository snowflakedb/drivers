"""
E2E tests for server-side error scenarios (real connection).

Tests verify that real Snowflake errors are surfaced as proper PEP 249 exceptions
with correct errno and sqlstate values.
"""

import uuid

import pytest

from snowflake.connector.errors import DatabaseError, Error, IntegrityError
from tests.compatibility import is_new_driver


PASSWORD_AUTH = "SNOWFLAKE_PASSWORD" if is_new_driver() else "snowflake"


class TestSQLQueryErrors:
    """Tests for SQL query errors against a real Snowflake connection."""

    def test_should_raise_database_error_for_malformed_sql(self, cursor):
        # When The user executes "SELEC 1"
        # Then DatabaseError is raised with errno 1003 and sqlstate "42000"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELEC 1")
        assert excinfo.value.errno == 1003
        assert excinfo.value.sqlstate == "42000"

    def test_should_raise_database_error_for_non_existent_table(self, cursor):
        # When The user executes "SELECT * FROM nonexistent_table_<random>"
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"

        # Then DatabaseError is raised with errno 2003
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name}")
        assert excinfo.value.errno == 2003

    def test_should_raise_database_error_for_non_existent_database(self, cursor):
        # When The user executes "USE DATABASE nonexistent_db_<random>"
        db_name = f"nonexistent_db_{uuid.uuid4().hex[:8]}"

        # Then DatabaseError is raised with errno 2043
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"USE DATABASE {db_name}")
        assert excinfo.value.errno == 2043


class TestIntegrityConstraintErrors:
    """Tests for integrity constraint violations."""

    def test_should_raise_integrity_error_for_null_in_not_null_column(self, cursor):
        # Given A temporary table with schema "id INT NOT NULL, name VARCHAR NOT NULL"
        table_name = f"test_notnull_{uuid.uuid4().hex[:8]}"
        cursor.execute(
            f"CREATE TEMPORARY TABLE {table_name} (id INT NOT NULL, name VARCHAR NOT NULL)"
        )

        try:
            # When The user executes "INSERT INTO t VALUES (1, null)"
            # Then IntegrityError is raised with errno 100072
            with pytest.raises(IntegrityError) as excinfo:
                cursor.execute(f"INSERT INTO {table_name} VALUES (1, null)")
            assert excinfo.value.errno == 100072
        finally:
            cursor.execute(f"DROP TABLE IF EXISTS {table_name}")

    def test_should_succeed_inserting_valid_values_into_not_null_columns(self, cursor):
        # Given A temporary table with schema "id INT NOT NULL, name VARCHAR NOT NULL"
        table_name = f"test_notnull_ok_{uuid.uuid4().hex[:8]}"
        cursor.execute(
            f"CREATE TEMPORARY TABLE {table_name} (id INT NOT NULL, name VARCHAR NOT NULL)"
        )

        try:
            # When The user executes "INSERT INTO t VALUES (1, 'Alice')"
            cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'Alice')")

            # Then No error is raised and rowcount is 1
            assert cursor.rowcount == 1
        finally:
            cursor.execute(f"DROP TABLE IF EXISTS {table_name}")


class TestAuthenticationErrors:
    """Tests for authentication error scenarios."""

    def test_should_raise_database_error_for_invalid_password(self, connection_factory):
        # When The user connects with an incorrect password
        # Then DatabaseError is raised with errno 250001
        with pytest.raises(DatabaseError) as excinfo:
            connection_factory(authenticator=PASSWORD_AUTH, password="wrong_password_12345")
        assert excinfo.value.errno == 250001

    def test_should_raise_error_for_non_existent_account(self, connection_factory):
        # When The user connects with account "nonexistent_account_<random>"
        account = f"nonexistent_account_{uuid.uuid4().hex[:8]}"

        # Then Error is raised with a non-default errno
        with pytest.raises(Error) as excinfo:
            connection_factory(
                account=account,
                authenticator=PASSWORD_AUTH,
                password="dummy",
            )
        assert excinfo.value.errno != -1
