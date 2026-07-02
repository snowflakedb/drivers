"""Connection constants shared by sync and async connection implementations."""

from __future__ import annotations

from typing import Any


# backward compatibility constant
# snowflake-sqlalchemy imports this symbol and calls .get(name) in
# parse_query_param_type to cast URL query-string values to the types the
# connector expects.  The universal driver validates parameters internally, so
# an empty dict is correct: every .get() returns None and values pass through
# uncast.
DEFAULT_CONFIGURATION: dict[str, tuple[Any, tuple[type, ...]]] = {}

APPLICATION_NAME = "PythonConnector"
# Kept as a public alias for backward compatibility — external packages
# (e.g. snowflake-sqlalchemy) may import this symbol.
CLIENT_NAME = APPLICATION_NAME

# Default upper bound for query strings included in log messages.  Mirrors
# the ``log_max_query_length`` default emitted by the generated
# :class:`ConnectionConfig` (sourced from ``PARAM_DEFS``); kept here as a
# named constant so the property fallback isn't a magic number.
LOG_MAX_QUERY_LENGTH = 80
