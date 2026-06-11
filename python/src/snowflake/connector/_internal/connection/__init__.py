"""Shared connection internals used by sync and async connection implementations."""

from __future__ import annotations

from ..connection_config_mixin import ConnectionConfigMixin, OptionsModifier
from .connection import ConnectionMixin
from .connection_types import ConnectionParameters, ConnectionParamValue, F, SessionParameters
from .constants import APPLICATION_NAME, CLIENT_NAME, DEFAULT_CONFIGURATION, LOG_MAX_QUERY_LENGTH
from .decorators import requires_open
from .queries import COMMIT_SQL, CURRENT_VERSION_SQL, ROLLBACK_SQL, SET_AUTOCOMMIT_SQL


__all__ = [
    "APPLICATION_NAME",
    "CLIENT_NAME",
    "COMMIT_SQL",
    "ConnectionMixin",
    "ConnectionConfigMixin",
    "ConnectionParamValue",
    "ConnectionParameters",
    "CURRENT_VERSION_SQL",
    "DEFAULT_CONFIGURATION",
    "F",
    "LOG_MAX_QUERY_LENGTH",
    "OptionsModifier",
    "ROLLBACK_SQL",
    "SessionParameters",
    "SET_AUTOCOMMIT_SQL",
    "requires_open",
]
