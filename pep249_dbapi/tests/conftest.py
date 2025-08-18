"""
pytest configuration and fixtures for PEP 249 tests.
"""

import pytest
import os
from typing import Any

from .connector_factory import ConnectorFactory, create_connection_with_adapter


def pytest_addoption(parser):
    """Add custom command line options to pytest."""
    parser.addoption(
        "--connector",
        action="store",
        default="new",
        choices=["new", "reference"],
        help="Which connector implementation to test against (default: new)"
    )
    parser.addoption(
        "--reference-package",
        action="store",
        default="snowflake.connector",
        help="Package name for reference connector (default: snowflake.connector)"
    )


@pytest.fixture(scope="session")
def connector_type(request):
    """Get the connector type from command line option."""
    return request.config.getoption("--connector")


@pytest.fixture(scope="session")
def connector_adapter(request, connector_type):
    """Create the appropriate connector adapter based on command line option."""
    reference_package = request.config.getoption("--reference-package")
    
    if connector_type == "reference":
        try:
            return ConnectorFactory.create_adapter("reference", package_name=reference_package)
        except ImportError as e:
            pytest.skip(f"Reference connector not available: {e}")
    else:
        return ConnectorFactory.create_adapter("new")


@pytest.fixture
def connection(connector_adapter):
    """Create a test connection using the configured connector adapter."""
    conn = create_connection_with_adapter(connector_adapter)
    yield conn
    # Cleanup: close connection if it has a close method
    if hasattr(conn, 'close') and callable(conn.close):
        try:
            conn.close()
        except Exception:
            pass  # Ignore cleanup errors


@pytest.fixture
def cursor(connection):
    """Create a test cursor from a connection."""
    cursor = connection.cursor()
    yield cursor
    # Cleanup: close cursor if it has a close method
    if hasattr(cursor, 'close') and callable(cursor.close):
        try:
            cursor.close()
        except Exception:
            pass  # Ignore cleanup errors


@pytest.fixture
def mock_connection():
    """Create a mock connection for tests that don't need real database connectivity."""
    from pep249_dbapi import Connection
    return Connection(database="testdb", user="testuser", password="testpass")


@pytest.fixture
def mock_cursor(mock_connection):
    """Create a mock cursor for tests that don't need real database connectivity."""
    return mock_connection.cursor()


def pytest_configure(config):
    """Configure pytest with custom markers."""
    config.addinivalue_line(
        "markers", 
        "integration: mark test as integration test requiring database connection"
    )
    config.addinivalue_line(
        "markers",
        "slow: mark test as slow (may take longer to run)"
    )


def pytest_collection_modifyitems(config, items):
    """Modify test collection to add markers automatically."""
    connector_type = config.getoption("--connector")
    
    # Add info about which connector is being tested
    print(f"\nTesting with connector: {connector_type}")
    
    for item in items:
        # Mark tests that use real connections as integration tests
        if any(fixture in item.fixturenames for fixture in ['connection', 'cursor']):
            if 'integration' not in [mark.name for mark in item.iter_markers()]:
                item.add_marker(pytest.mark.integration)