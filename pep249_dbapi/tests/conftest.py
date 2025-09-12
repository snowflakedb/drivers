"""
pytest configuration and fixtures for PEP 249 tests.
"""

import pytest

from .connector_factory import ConnectorFactory, create_connection_with_adapter
from .utils import set_current_connector
from .connector_types import ConnectorType


def pytest_addoption(parser):
    """Add custom command line options to pytest."""
    parser.addoption(
        "--connector",
        action="store",
        default="universal",
        choices=["universal", "reference"],
        help="Which connector implementation to test against (default: universal)"
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
    connector_str = request.config.getoption("--connector")
    return ConnectorType.from_string(connector_str)


@pytest.fixture(scope="session")
def connector_adapter(request, connector_type):
    """Create the appropriate connector adapter based on command line option."""
    reference_package = request.config.getoption("--reference-package")
    
    try:
        if connector_type == ConnectorType.REFERENCE:
            return ConnectorFactory.create_adapter(connector_type, package_name=reference_package)
        else:
            return ConnectorFactory.create_adapter(connector_type)
    except ImportError as e:
        pytest.skip(f"Connector {connector_type} not available: {e}")


@pytest.fixture
def connection(connector_adapter):
    """Create a test connection using the configured connector adapter."""
    with create_connection_with_adapter(connector_adapter) as conn:
        yield conn


@pytest.fixture
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


def pytest_runtest_setup(item):
    """Skip tests based on connector type and markers."""
    connector_type = item.config.getoption("--connector")
    # Set the current connector for driver-gated helpers
    set_current_connector(connector_type)
    
    if connector_type == "universal" and item.get_closest_marker("skip_universal"):
        pytest.skip("Skipping test for universal driver")
    elif connector_type == "reference" and item.get_closest_marker("skip_reference"):
        pytest.skip("Skipping test for reference driver")