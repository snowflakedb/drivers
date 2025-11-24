"""
Connector factory for testing different Snowflake connector implementations.

This module provides a unified interface to test different Snowflake connector
implementations with the same test suite.
"""

from abc import ABC, abstractmethod
from typing import Any, Dict
import importlib

from .compatibility import is_new_driver, is_old_driver
from .config import get_test_parameters
from .connector_types import ConnectorType
from .private_key_helper import get_private_key_from_parameters, get_private_key_password


class ConnectorAdapter(ABC):
    """Abstract base class for connector adapters."""
    
    @abstractmethod
    def connect(self, **kwargs) -> Any:
        """Create a connection using this connector implementation."""
        pass
    
    @property
    @abstractmethod
    def name(self) -> str:
        """Return the name of this connector implementation."""
        pass
    
    @property
    @abstractmethod
    def version(self) -> str:
        """Return the version of this connector implementation."""
        pass
    
    @property
    @abstractmethod
    def connector_type(self) -> ConnectorType:
        """Return the connector type enum."""
        pass


class UniversalConnectorAdapter(ConnectorAdapter):
    """Adapter for the universal driver implementation."""
    
    def __init__(self):
        # Import the universal connector
        from snowflake import ud_connector
        self.connector = ud_connector
    
    def connect(self, **kwargs) -> Any:
        """Create a connection using the universal connector."""
        return self.connector.connect(**kwargs)
    
    @property
    def name(self) -> str:
        return "pep249_dbapi"
    
    @property
    def version(self) -> str:
        try:
            return self.connector.__version__
        except AttributeError:
            return "0.1.0"
    
    @property
    def connector_type(self) -> ConnectorType:
        return ConnectorType.UNIVERSAL


class ReferenceConnectorAdapter(ConnectorAdapter):
    """Adapter for the reference Snowflake connector implementation."""
    
    def __init__(self, package_name: str = "snowflake.connector"):
        self.package_name = package_name
        try:
            self.connector = importlib.import_module(package_name)
        except ImportError as e:
            raise ImportError(f"Could not import reference connector '{package_name}': {e}")
    
    def connect(self, **kwargs) -> Any:
        """Create a connection using the reference connector."""
        return self.connector.connect(**kwargs)
    
    @property
    def name(self) -> str:
        return self.package_name
    
    @property
    def version(self) -> str:
        try:
            return self.connector.__version__
        except AttributeError:
            return "unknown"
    
    @property
    def connector_type(self) -> ConnectorType:
        return ConnectorType.REFERENCE


class ConnectorFactory:
    """Factory for creating connector adapters."""
    
    _adapters = {
        ConnectorType.UNIVERSAL: UniversalConnectorAdapter,
        ConnectorType.REFERENCE: ReferenceConnectorAdapter,
    }
    
    @classmethod
    def create_adapter(cls, connector_type: ConnectorType, **kwargs) -> ConnectorAdapter:
        """Create a connector adapter of the specified type."""
        if connector_type not in cls._adapters:
            raise ValueError(f"Unknown connector type: {connector_type}. "
                           f"Available types: {list(cls._adapters.keys())}")
        
        adapter_class = cls._adapters[connector_type]
        return adapter_class(**kwargs)
    
    @classmethod
    def get_available_connectors(cls) -> Dict[ConnectorType, str]:
        """Get a list of available connector types and their descriptions."""
        return {
            ConnectorType.UNIVERSAL: "Universal driver implementation",
            ConnectorType.REFERENCE: "Old Snowflake connector implementation"
        }


def create_connection_with_adapter(adapter: ConnectorAdapter, **override_params):
    """Create a connection using the specified adapter and test parameters.

    Args:
        adapter: The connector adapter to use
        **override_params: Parameters to override defaults (e.g., account="test", user="testuser")
    """
    test_params = get_test_parameters()
    
    # Convert test parameter names to connection parameter names
    connection_params = {
        "account": test_params.get("SNOWFLAKE_TEST_ACCOUNT"),
        "user": test_params.get("SNOWFLAKE_TEST_USER"),
        "database": test_params.get("SNOWFLAKE_TEST_DATABASE"),
        "schema": test_params.get("SNOWFLAKE_TEST_SCHEMA"),
        "warehouse": test_params.get("SNOWFLAKE_TEST_WAREHOUSE"),
        "role": test_params.get("SNOWFLAKE_TEST_ROLE"),
    }
    
    # Use JWT authentication by default (unless custom private_key_file or authenticator is provided)
    if "private_key_file" not in override_params and "authenticator" not in override_params:
        setup_default_jwt_auth(connection_params)
    
    # Add optional parameters if they exist
    if test_params.get("SNOWFLAKE_TEST_SERVER_URL"):
        connection_params["server_url"] = test_params["SNOWFLAKE_TEST_SERVER_URL"]
    if test_params.get("SNOWFLAKE_TEST_HOST"):
        connection_params["host"] = test_params["SNOWFLAKE_TEST_HOST"]
    if test_params.get("SNOWFLAKE_TEST_PORT"):
        connection_params["port"] = test_params["SNOWFLAKE_TEST_PORT"]
    if test_params.get("SNOWFLAKE_TEST_PROTOCOL"):
        connection_params["protocol"] = test_params["SNOWFLAKE_TEST_PROTOCOL"]
    
    # Remove None values
    connection_params = {k: v for k, v in connection_params.items() if v is not None}
    
    # Apply overrides
    connection_params.update(override_params)
    
    return adapter.connect(**connection_params)


def setup_default_jwt_auth(connection_params: Dict[str, Any]) -> None:
    """Set up default JWT authentication using encrypted private key from environment.
    
    Args:
        connection_params: Dictionary to populate with JWT auth parameters
    """
    connection_params["authenticator"] = "SNOWFLAKE_JWT"
    private_key_path = get_private_key_from_parameters()
    connection_params["private_key_file"] = private_key_path
    
    private_key_pwd = get_private_key_password()
    if private_key_pwd:
        if is_old_driver():
            connection_params["private_key_file_pwd"] = private_key_pwd
        elif is_new_driver():
            connection_params["private_key_password"] = private_key_pwd
