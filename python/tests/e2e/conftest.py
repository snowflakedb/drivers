import pytest


@pytest.fixture(scope="session")
def tmp_schema(connection_factory):
    """Single schema shared across the e2e test session.

    Overrides the function-scoped tmp_schema from the top-level conftest so
    that e2e tests pay the CREATE SCHEMA / DROP SCHEMA cost once per session
    instead of once per test.

    Uses a dedicated connection so schema lifecycle does not compete with test
    queries. Tests should generally use CREATE OR REPLACE TEMPORARY TABLE
    within this schema to avoid name conflicts; these temporary tables are
    dropped when the creating connection is closed (typically at end of each
    test). Some tests may intentionally create non-temporary tables for DDL
    coverage.
    """
    import uuid

    schema_name = f"test_schema_{uuid.uuid4().hex}"
    with connection_factory() as schema_conn:
        with schema_conn.cursor() as cur:
            cur.execute(f"CREATE SCHEMA {schema_name}")
    try:
        yield schema_name
    finally:
        with connection_factory() as schema_conn:
            with schema_conn.cursor() as cur:
                cur.execute(f"DROP SCHEMA IF EXISTS {schema_name} CASCADE")
