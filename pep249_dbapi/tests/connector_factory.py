"""
Connector factory for testing different Snowflake connector implementations.

This module provides a unified interface to test different Snowflake connector
implementations with the same test suite.
"""

import os
import json
from abc import ABC, abstractmethod
from typing import Dict, Any
import importlib

from .connector_types import ConnectorType


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
        import pep249_dbapi
        self.connector = pep249_dbapi
    
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


def get_test_parameters():
    """Get test connection parameters from environment or parameters file."""
    # First try environment variable
    parameter_path = os.environ.get("PARAMETER_PATH")
    if parameter_path and os.path.exists(parameter_path):
        with open(parameter_path) as f:
            parameters = json.load(f)
            return parameters.get("testconnection", {})
    
    # Fallback to default test parameters (for local testing)
    return {
        "SNOWFLAKE_TEST_ACCOUNT": os.environ.get("SNOWFLAKE_TEST_ACCOUNT"),
        "SNOWFLAKE_TEST_USER": os.environ.get("SNOWFLAKE_TEST_USER"),
        "SNOWFLAKE_TEST_PASSWORD": os.environ.get("SNOWFLAKE_TEST_PASSWORD"),
        "SNOWFLAKE_TEST_DATABASE": os.environ.get("SNOWFLAKE_TEST_DATABASE"),
        "SNOWFLAKE_TEST_SCHEMA": os.environ.get("SNOWFLAKE_TEST_SCHEMA"),
        "SNOWFLAKE_TEST_WAREHOUSE": os.environ.get("SNOWFLAKE_TEST_WAREHOUSE"),
        "SNOWFLAKE_TEST_ROLE": os.environ.get("SNOWFLAKE_TEST_ROLE"),
        "SNOWFLAKE_TEST_SERVER_URL": os.environ.get("SNOWFLAKE_TEST_SERVER_URL"),
        "SNOWFLAKE_TEST_HOST": os.environ.get("SNOWFLAKE_TEST_HOST"),
        "SNOWFLAKE_TEST_PORT": os.environ.get("SNOWFLAKE_TEST_PORT"),
        "SNOWFLAKE_TEST_PROTOCOL": os.environ.get("SNOWFLAKE_TEST_PROTOCOL"),
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
        "password": test_params.get("SNOWFLAKE_TEST_PASSWORD"),
        "database": test_params.get("SNOWFLAKE_TEST_DATABASE"),
        "schema": test_params.get("SNOWFLAKE_TEST_SCHEMA"),
        "warehouse": test_params.get("SNOWFLAKE_TEST_WAREHOUSE"),
        "role": test_params.get("SNOWFLAKE_TEST_ROLE"),
    }
    
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
