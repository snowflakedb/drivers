"""
PEP 249 Database API 2.0 Connection Objects

This package defines the Connection class as specified in PEP 249.
"""

from __future__ import annotations

from .._internal.connection.constants import (
    APPLICATION_NAME as _APPLICATION_NAME,
)
from .._internal.connection.constants import (
    CLIENT_NAME,
    DEFAULT_CONFIGURATION,
    LOG_MAX_QUERY_LENGTH,
)
from ..connection_config import ConnectionConfig, OptionsModifier
from ._connection import Connection, SnowflakeConnection


__all__ = [
    "CLIENT_NAME",
    "Connection",
    "ConnectionConfig",
    "DEFAULT_CONFIGURATION",
    "LOG_MAX_QUERY_LENGTH",
    "OptionsModifier",
    "SnowflakeConnection",
    "_APPLICATION_NAME",
]
