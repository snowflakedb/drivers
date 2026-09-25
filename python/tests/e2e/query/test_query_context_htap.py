"""
E2E tests: HTAP optimization preserves session metadata.

When ENABLE_SNOW_654741_FOR_TESTING is set, the server omits parameters and
metadata from SELECT responses. The driver must still report the correct
database, schema, warehouse, role, and session parameters from its client-side
cache.
"""

from __future__ import annotations

import datetime
import time

import pytest

from snowflake.connector.errors import ProgrammingError
from tests.e2e.put_get.put_get_helper import is_aws_test_account


HYBRID_TABLE_DB_LIMIT_ERRNO = 391727
HYBRID_TEST_DB_PREFIX = "HYBRID_DB_TEST_"


def _is_hybrid_table_db_limit(exc: BaseException) -> bool:
    return isinstance(exc, ProgrammingError) and exc.errno == HYBRID_TABLE_DB_LIMIT_ERRNO


def _drop_database_quiet(cur, name: str) -> None:
    try:
        cur.execute("DROP DATABASE IF EXISTS IDENTIFIER(?)", params=(name,), _force_qmark_paramstyle=True)
    except ProgrammingError:
        pass


def _drop_stale_hybrid_test_databases(cur, *, max_age_seconds: int = 15 * 60) -> None:
    """Drop leftover hybrid_db_test_* databases from crashed CI jobs.

    Concurrent runs already use a unique millisecond timestamp in the name.
    Only drop databases older than max_age_seconds so in-flight tests survive.
    """
    cur.execute(f"SHOW DATABASES LIKE '{HYBRID_TEST_DB_PREFIX}%'")
    rows = cur.fetchall()
    now = datetime.datetime.now(datetime.UTC)
    for row in rows:
        created_on, name = row[0], row[1]
        if isinstance(created_on, datetime.datetime):
            created = created_on if created_on.tzinfo else created_on.replace(tzinfo=datetime.UTC)
            if (now - created.astimezone(datetime.UTC)).total_seconds() < max_age_seconds:
                continue
        _drop_database_quiet(cur, name)


def _execute_with_hybrid_quota_retry(cur, sql: str, *, params: tuple | None = None) -> None:
    """Run sql; on 391727 drop stale hybrid_db_test_* DBs, retry once, else skip.

    391727 is a quota on databases *with hybrid tables*, not on CREATE DATABASE.
    Empty DBs do not count, so CREATE HYBRID TABLE is the statement that fails.
    """
    extra: dict[str, object] = {}
    if params is not None:
        extra["params"] = params
        extra["_force_qmark_paramstyle"] = True
    try:
        cur.execute(sql, **extra)
        return
    except ProgrammingError as exc:
        if not _is_hybrid_table_db_limit(exc):
            raise
    _drop_stale_hybrid_test_databases(cur)
    try:
        cur.execute(sql, **extra)
    except ProgrammingError as retry_exc:
        if _is_hybrid_table_db_limit(retry_exc):
            pytest.skip(
                "account hybrid-table database quota (391727) is exhausted after "
                "cleaning stale hybrid_db_test_* databases"
            )
        raise


def _create_hybrid_test_database(cur, name: str) -> None:
    _execute_with_hybrid_quota_retry(
        cur,
        "CREATE DATABASE IF NOT EXISTS IDENTIFIER(?)",
        params=(name,),
    )


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
                    # 391727 counts databases that already contain a hybrid table,
                    # not CREATE DATABASE. Drain leftovers before we consume a slot.
                    _drop_stale_hybrid_test_databases(cur)
                    # When the client creates hybrid tables in two databases and inserts rows
                    _create_hybrid_test_database(cur, db1)
                    _execute_with_hybrid_quota_retry(
                        cur,
                        "CREATE HYBRID TABLE test_hybrid_table (id INT PRIMARY KEY, text VARCHAR)",
                    )
                    cur.execute("INSERT INTO test_hybrid_table VALUES (1, 'a')")

                    rows = cur.execute("SELECT * FROM test_hybrid_table").fetchall()
                    assert rows == [(1, "a")]

                    cur.execute("INSERT INTO test_hybrid_table VALUES (2, 'b')")
                    rows = cur.execute("SELECT * FROM test_hybrid_table ORDER BY id").fetchall()
                    assert rows == [(1, "a"), (2, "b")]

                    _create_hybrid_test_database(cur, db2)
                    _execute_with_hybrid_quota_retry(
                        cur,
                        "CREATE HYBRID TABLE test_hybrid_table_2 (id INT PRIMARY KEY, text VARCHAR)",
                    )
                    cur.execute("INSERT INTO test_hybrid_table_2 VALUES (3, 'c')")

                    rows = cur.execute("SELECT * FROM test_hybrid_table_2").fetchall()
                    assert rows == [(3, "c")]

                    cur.execute(
                        "USE DATABASE IDENTIFIER(?)",
                        params=(db1,),
                        _force_qmark_paramstyle=True,
                    )
                    cur.execute("INSERT INTO test_hybrid_table VALUES (4, 'd')")

                    # Then selecting from each database returns the correct rows after switching back
                    rows = cur.execute("SELECT * FROM test_hybrid_table ORDER BY id").fetchall()
                    assert len(rows) == 3
                    assert rows[0] == (1, "a")
                    assert rows[1] == (2, "b")
                    assert rows[2] == (4, "d")
                finally:
                    # Drop first: USE DATABASE must not skip cleanup if original_db is gone.
                    _drop_database_quiet(cur, db1)
                    _drop_database_quiet(cur, db2)
                    if original_db:
                        try:
                            cur.execute(
                                "USE DATABASE IDENTIFIER(?)",
                                params=(original_db,),
                                _force_qmark_paramstyle=True,
                            )
                        except ProgrammingError:
                            pass
