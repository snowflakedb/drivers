"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from ._internal.snowflake_restful import SnowflakeRestful  # noqa: F401


class ReauthenticationRequest(Exception):
    def __init__(self, cause: Exception | None = None) -> None:
        self.cause = cause
