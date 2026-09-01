"""
E2E tests: HTAP optimization preserves session metadata.

When ENABLE_SNOW_654741_FOR_TESTING is set, the server omits parameters and
metadata from SELECT responses. The driver must still report the correct
database, schema, warehouse, role, and session parameters from its client-side
cache.
"""

from __future__ import annotations

import time

import pytest

from tests.e2e.put_get.put_get_helper import is_aws_test_account


@pytest.fixture
def htap_connection(connection_factory):
    """Yield a connection with ENABLE_SNOW_654741_FOR_TESTING enabled."""
    with connection_factory() as conn:
        with conn.cursor() as cur:
            cur.execute("ALTER SESSION SET ENABLE_SNOW_654741_FOR_TESTING = true")
        yield conn


class TestQueryContextHtap:
    # Scenario: should preserve schema after SELECT under HTAP optimization
    def test_should_preserve_schema_after_select_under_htap_optimization(
        self,
        htap_connection,
    ):
        conn = htap_connection
        # Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
        run_id = int(time.time() * 1000)
        new_schema = f"test_schema_{run_id}"
        assert conn.schema is None or not conn.schema.upper() == new_schema.upper()

        # When the client creates a new schema and executes SELECT
        with conn.cursor() as cur:
            cur.execute(f"CREATE SCHEMA {new_schema}")
            try:
                assert conn.schema.upper() == new_schema.upper()

                cur.execute("SELECT 1")

                # Then the connection still reports the new schema
                assert conn.schema.upper() == new_schema.upper(), (
                    f"Schema should survive SELECT, expected {new_schema}, got {conn.schema}"
                )
            finally:
                cur.execute(f"DROP SCHEMA IF EXISTS {new_schema}")

    # Scenario: should preserve database after SELECT under HTAP optimization
    def test_should_preserve_database_after_select_under_htap_optimization(
        self,
        htap_connection,
    ):
        conn = htap_connection
        # Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
        run_id = int(time.time() * 1000)
        new_database = f"test_database_{run_id}"
        assert conn.database is None or not conn.database.upper() == new_database.upper()

        # When the client creates a new database and executes SELECT
        with conn.cursor() as cur:
            cur.execute(f"CREATE DATABASE {new_database}")
            try:
                assert conn.database.upper() == new_database.upper()

                cur.execute("SELECT 1")

                # Then the connection still reports the new database
                assert conn.database.upper() == new_database.upper(), (
                    f"Database should survive SELECT, expected {new_database}, got {conn.database}"
                )
            finally:
                cur.execute(f"DROP DATABASE IF EXISTS {new_database}")

    # Scenario: should preserve role after SELECT under HTAP optimization
    def test_should_preserve_role_after_select_under_htap_optimization(
        self,
        htap_connection,
    ):
        conn = htap_connection
        # Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
        assert conn.role is None or not conn.role.upper() == "PUBLIC"

        # When the client switches to a different role and executes SELECT
        with conn.cursor() as cur:
            cur.execute("USE ROLE PUBLIC")
            assert conn.role.upper() == "PUBLIC"

            cur.execute("SELECT 1")

            # Then the connection still reports the switched role
            assert conn.role.upper() == "PUBLIC", f"Role should survive SELECT, expected PUBLIC, got {conn.role}"

    # Scenario: should preserve session parameter after SELECT under HTAP optimization
    def test_should_preserve_session_parameter_after_select_under_htap_optimization(
        self,
        htap_connection,
    ):
        conn = htap_connection
        # Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
        with conn.cursor() as cur:
            # When the client changes DATE_OUTPUT_FORMAT and executes SELECT
            cur.execute("ALTER SESSION SET DATE_OUTPUT_FORMAT = 'DD-MM-YYYY'")
            try:
                assert conn._session_parameters["DATE_OUTPUT_FORMAT"] == "DD-MM-YYYY"

                cur.execute("SELECT 1")

                # Then the session parameter still reflects the changed value
                assert conn._session_parameters["DATE_OUTPUT_FORMAT"] == "DD-MM-YYYY", (
                    f"DATE_OUTPUT_FORMAT should survive SELECT, got {conn._session_parameters['DATE_OUTPUT_FORMAT']}"
                )
            finally:
                cur.execute("ALTER SESSION SET DATE_OUTPUT_FORMAT = 'YYYY-MM-DD'")

    # Scenario: should operate on hybrid tables across multiple databases
    @pytest.mark.skipif(not is_aws_test_account(), reason="HTAP hybrid tables are enabled only on AWS")
    def test_should_operate_on_hybrid_tables_across_multiple_databases(
        self,
        connection_factory,
    ):
        # Given a connection to Snowflake
        run_id = int(time.time() * 1000)
        db1 = f"hybrid_db_test_{run_id}"
        db2 = f"hybrid_db_test_{run_id}_2"

        with connection_factory() as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT CURRENT_DATABASE()")
                original_db = cur.fetchone()[0]

                try:
                    # When the client creates hybrid tables in two databases and inserts rows
                    cur.execute(f"CREATE DATABASE IF NOT EXISTS {db1}")
                    cur.execute("CREATE HYBRID TABLE test_hybrid_table (id INT PRIMARY KEY, text VARCHAR)")
                    cur.execute("INSERT INTO test_hybrid_table VALUES (1, 'a')")

                    rows = cur.execute("SELECT * FROM test_hybrid_table").fetchall()
                    assert rows == [(1, "a")]

                    cur.execute("INSERT INTO test_hybrid_table VALUES (2, 'b')")
                    rows = cur.execute("SELECT * FROM test_hybrid_table ORDER BY id").fetchall()
                    assert rows == [(1, "a"), (2, "b")]

                    cur.execute(f"CREATE DATABASE IF NOT EXISTS {db2}")
                    cur.execute("CREATE HYBRID TABLE test_hybrid_table_2 (id INT PRIMARY KEY, text VARCHAR)")
                    cur.execute("INSERT INTO test_hybrid_table_2 VALUES (3, 'c')")

                    rows = cur.execute("SELECT * FROM test_hybrid_table_2").fetchall()
                    assert rows == [(3, "c")]

                    cur.execute(f"USE DATABASE {db1}")
                    cur.execute("INSERT INTO test_hybrid_table VALUES (4, 'd')")

                    # Then selecting from each database returns the correct rows after switching back
                    rows = cur.execute("SELECT * FROM test_hybrid_table ORDER BY id").fetchall()
                    assert len(rows) == 3
                    assert rows[0] == (1, "a")
                    assert rows[1] == (2, "b")
                    assert rows[2] == (4, "d")
                finally:
                    cur.execute(f"USE DATABASE {original_db}")
                    cur.execute(f"DROP DATABASE IF EXISTS {db1}")
                    cur.execute(f"DROP DATABASE IF EXISTS {db2}")
