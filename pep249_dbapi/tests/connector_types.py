"""
Connector type definitions for testing framework.
"""

from enum import Enum


class ConnectorType(Enum):
    """Enumeration of supported connector types for testing."""
    
    UNIVERSAL = "universal"  # Universal driver implementation
    REFERENCE = "reference"  # Old Snowflake Python connector
    
    def __str__(self) -> str:
        return self.value
    
    @classmethod
    def from_string(cls, value: str) -> 'ConnectorType':
        """Create ConnectorType from string value."""
        for connector_type in cls:
            if connector_type.value == value:
                return connector_type
        raise ValueError(f"Unknown connector type: {value}. Available: {[t.value for t in cls]}")
