"""
pytest configuration and fixtures for PEP 249 tests.
"""

from __future__ import annotations

from typing import Any
from urllib.parse import urlparse

import pytest

from snowflake.connector.cursor import DictCursor

from .compatibility import IS_UNIVERSAL_DRIVER
from .connector_factory import ConnectorFactory, create_connection_with_adapter
from .private_key_helper import get_test_private_key_path


# Type alias for a single row returned from cursor
Row = tuple[Any, ...]


@pytest.mark.optionalhook
def pytest_metadata(metadata):
    metadata["Version of snowflake.connector"] = "Universal" if IS_UNIVERSAL_DRIVER else "Old"


@pytest.fixture(scope="session")
def connector_adapter(request):
    return ConnectorFactory.create_adapter()


@pytest.fixture
def connection(connector_adapter):
    """Create a test connection using the configured connector adapter."""
    with create_connection_with_adapter(connector_adapter) as conn:
        yield conn


@pytest.fixture(scope="session")
def connection_factory(connector_adapter):
    """Factory function for creating connections with custom parameters."""

    def _create_connection(**override_params):
        """Create a connection with custom parameters.

        Args:
            **override_params: Parameters to override defaults

        Example:
            conn = connection_factory(account="test_account", user="test_user")
        """
        return create_connection_with_adapter(connector_adapter, **override_params)

    return _create_connection


@pytest.fixture
def cursor(connection):
    """Create a test cursor from a connection."""
    with connection.cursor() as cursor:
        yield cursor


@pytest.fixture
def dict_cursor(connection):
    """Create a DictCursor from a connection."""
    with connection.cursor(cursor_class=DictCursor) as cursor:
        yield cursor


@pytest.fixture
def execute_query(cursor):
    """Helper replacing cursor if your only use case is to execute a query."""

    def _execute_query(*args: Any, single_row: bool = False, **kwargs: Any) -> Row | list[Row] | None:
        cursor.execute(*args, **kwargs)
        if single_row:
            return cursor.fetchone()
        return cursor.fetchall()

    return _execute_query


@pytest.fixture
def tmp_schema(cursor):
    """Create a temporary schema."""
    import uuid

    schema_name = f"test_schema_{uuid.uuid4().hex}"
    cursor.execute(f"CREATE SCHEMA {schema_name}")
    try:
        yield schema_name
    finally:
        cursor.execute(f"DROP SCHEMA {schema_name}")


@pytest.fixture
def int_test_connection_factory(connector_adapter):
    """Factory function for creating connections with integration test parameters."""

    def _create_connection(**override_params):
        """Create a connection with integration test parameters."""
        default_server_url = "http://localhost:8090"
        server_url = override_params.get("server_url", default_server_url)
        parsed_url = urlparse(server_url)

        # Default integration test parameters
        integration_params = {
            "account": "test_account",
            "user": "test_user",
            "database": "test_database",
            "schema": "test_schema",
            "warehouse": "test_warehouse",
            "role": "test_role",
            "server_url": server_url,
            "protocol": parsed_url.scheme,
            "host": parsed_url.hostname,
            "port": parsed_url.port,
            "authenticator": "SNOWFLAKE_JWT",
            "private_key_file": get_test_private_key_path(),
        }

        integration_params.update(override_params)

        return create_connection_with_adapter(connector_adapter, **integration_params)

    return _create_connection


def pytest_runtest_setup(item):
    """Skip tests based on connector type and markers."""
    if IS_UNIVERSAL_DRIVER and item.get_closest_marker("skip_universal"):
        pytest.skip("Skipping test for universal driver")
    elif not IS_UNIVERSAL_DRIVER and item.get_closest_marker("skip_reference"):
        pytest.skip("Skipping test for reference driver")
