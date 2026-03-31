"""
Integration tests: connection properties must return server-echoed names.

conn.database, conn.schema, etc. must reflect the canonical name returned
by the server — not the raw value passed to connect() or the value from
before a USE statement.

We use identifiers containing dashes (e.g. ``my-db``) as test fixtures
because they require SQL quoting, making discrepancies between the raw
user input and the server-echoed name easy to detect.
"""

from __future__ import annotations

import uuid

import pytest


def _unique_name(base: str) -> str:
    """Generate a unique identifier with the given base and a UUID suffix."""
    return f"{base}_{uuid.uuid4().hex[:8]}"


@pytest.fixture
def dashed_database(cursor):
    """Create a temporary database whose name contains a dash.

    Dashes force SQL quoting (``"my-db"``), making it easy to detect when
    the driver returns the raw user input instead of the server-echoed name.
    """
    cursor.execute("SELECT CURRENT_DATABASE()")
    original_db = cursor.fetchone()[0]

    db_name = _unique_name("test-dashed-db")
    cursor.execute(f'CREATE DATABASE "{db_name}"')
    try:
        yield db_name
    finally:
        if original_db:
            cursor.execute(f'USE DATABASE "{original_db}"')
        cursor.execute(f'DROP DATABASE IF EXISTS "{db_name}"')


@pytest.fixture
def dashed_schema(cursor):
    """Create a temporary schema whose name contains a dash.

    See ``dashed_database`` for why dashes are used.
    """
    cursor.execute("SELECT CURRENT_SCHEMA()")
    row = cursor.fetchone()
    original_schema = row[0] if row and row[0] is not None else None

    schema_name = _unique_name("test-dashed-schema")
    cursor.execute(f'CREATE SCHEMA "{schema_name}"')
    try:
        yield schema_name
    finally:
        if original_schema:
            cursor.execute(f'USE SCHEMA "{original_schema}"')
        cursor.execute(f'DROP SCHEMA IF EXISTS "{schema_name}"')


class TestConnectionPropertyReflectsUse:
    """conn.database/schema must update after USE statements."""

    def test_database_updates_after_use(self, connection, dashed_database):
        """conn.database reflects the new database after USE DATABASE."""
        with connection.cursor() as cur:
            cur.execute(f'USE DATABASE "{dashed_database}"')

        db = connection.database
        assert db is not None
        assert db.lower() == dashed_database.lower(), (
            f"conn.database did not update after USE DATABASE: expected {dashed_database!r}, got {db!r}"
        )

    def test_schema_updates_after_use(self, connection, dashed_schema):
        """conn.schema reflects the new schema after USE SCHEMA."""
        with connection.cursor() as cur:
            cur.execute(f'USE SCHEMA "{dashed_schema}"')

        schema = connection.schema
        assert schema is not None
        assert schema.lower() == dashed_schema.lower(), (
            f"conn.schema did not update after USE SCHEMA: expected {dashed_schema!r}, got {schema!r}"
        )


class TestConnectionPropertyIsServerEchoed:
    """conn.database/schema must return server-echoed names, not raw input."""

    def test_database_at_connect_returns_server_name(self, connection_factory, dashed_database, cursor):
        """conn.database returns the server-echoed name, not raw connect() input."""
        cursor.execute(f'USE DATABASE "{dashed_database}"')

        with connection_factory(database=dashed_database) as conn:
            db = conn.database
            assert db is not None
            assert '"' not in db, (
                f"conn.database contains quotes: {db!r}. Expected the server-echoed name without SQL quotes."
            )
            assert db.lower() == dashed_database.lower()

    def test_quoted_database_param_returns_server_name(self, connection_factory, dashed_database, cursor):
        """When connect() receives a pre-quoted value, conn.database still returns the unquoted server name."""
        cursor.execute(f'USE DATABASE "{dashed_database}"')

        quoted_value = f'"{dashed_database}"'
        with connection_factory(database=quoted_value) as conn:
            db = conn.database
            assert db is not None
            assert '"' not in db, (
                f"conn.database returned raw input {db!r} instead of "
                f"the server-echoed name for database={quoted_value!r}."
            )
            assert db.lower() == dashed_database.lower()


class TestConnectionPropertyMatchesServerCase:
    """conn.database/schema must match the exact case returned by the server."""

    def test_database_case_matches_server(self, connection):
        """conn.database has the same case as CURRENT_DATABASE()."""
        with connection.cursor() as cur:
            cur.execute("SELECT CURRENT_DATABASE()")
            server_db = cur.fetchone()[0]

        assert connection.database == server_db, (
            f"conn.database case mismatch: driver returned {connection.database!r}, server returned {server_db!r}"
        )

    def test_schema_case_matches_server(self, connection):
        """conn.schema has the same case as CURRENT_SCHEMA()."""
        with connection.cursor() as cur:
            cur.execute("SELECT CURRENT_SCHEMA()")
            server_schema = cur.fetchone()[0]

        assert connection.schema == server_schema, (
            f"conn.schema case mismatch: driver returned {connection.schema!r}, server returned {server_schema!r}"
        )

    def test_schema_stable_after_query(self, connection):
        """conn.schema is unchanged (exact case) after an unrelated query."""
        schema_before = connection.schema
        assert schema_before is not None
        assert schema_before != ""

        with connection.cursor() as cur:
            cur.execute("SELECT 1")

        assert connection.schema == schema_before, (
            f"conn.schema changed after a simple query: was {schema_before!r}, now {connection.schema!r}"
        )


class TestConnectionPropertyRoundtrip:
    """conn.database/schema value must be usable in subsequent SQL."""

    def test_database_property_usable_in_sql(self, connection, dashed_database):
        """Reading conn.database after USE and using it in a new USE statement succeeds."""
        with connection.cursor() as cur:
            cur.execute(f'USE DATABASE "{dashed_database}"')
            db = connection.database
            assert db is not None

            # This is the pattern that breaks when conn.database returns
            # raw input with embedded quotes instead of the server name.
            cur.execute(f'USE DATABASE "{db}"')

        assert connection.database is not None
        assert connection.database.lower() == db.lower()

    def test_schema_property_usable_in_sql(self, connection, dashed_schema):
        """Reading conn.schema after USE and using it in a new USE statement succeeds."""
        with connection.cursor() as cur:
            cur.execute(f'USE SCHEMA "{dashed_schema}"')
            schema = connection.schema
            assert schema is not None

            cur.execute(f'USE SCHEMA "{schema}"')

        assert connection.schema is not None
        assert connection.schema.lower() == schema.lower()
