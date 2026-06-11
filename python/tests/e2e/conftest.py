import pytest

from tests.connector_factory import create_connection_with_adapter


@pytest.fixture(scope="session")
def tmp_schema(connector_adapter):
    """Single schema shared across the e2e test session.

    Overrides the function-scoped tmp_schema from the top-level conftest so
    that e2e tests pay the CREATE SCHEMA / DROP SCHEMA cost once per session
    instead of once per test.

    Uses connector_adapter (session-scoped) directly rather than
    connection_factory (module-scoped) to avoid a scope mismatch.

    Tests should generally use CREATE OR REPLACE TEMPORARY TABLE within this
    schema to avoid name conflicts; these temporary tables are dropped when the
    creating connection is closed (typically at end of each test).
    """
    import uuid

    schema_name = f"test_schema_{uuid.uuid4().hex}"
    with create_connection_with_adapter(connector_adapter) as schema_conn:
        with schema_conn.cursor() as cur:
            cur.execute(f"CREATE SCHEMA {schema_name}")
    try:
        yield schema_name
    finally:
        with create_connection_with_adapter(connector_adapter) as schema_conn:
            with schema_conn.cursor() as cur:
                cur.execute(f"DROP SCHEMA IF EXISTS {schema_name} CASCADE")
