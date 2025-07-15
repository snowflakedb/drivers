"""
pytest configuration and fixtures for PEP 249 tests.
"""

import pytest
from pep249_dbapi import Connection, Cursor


@pytest.fixture
def connection():
    """Create a test connection."""
    return Connection(database="testdb", user="testuser", password="testpass")


@pytest.fixture
def cursor(connection):
    """Create a test cursor from a connection."""
    return connection.cursor()


@pytest.fixture
def closed_connection():
    """Create a closed connection for testing."""
    conn = Connection()
    conn.close()
    return conn 