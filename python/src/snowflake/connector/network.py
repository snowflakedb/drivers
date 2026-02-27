"""BACKWARD COMPATIBILITY MODULE ONLY"""

from ._internal.snowflake_restful import SnowflakeRestful  # noqa: F401


class ReauthenticationRequest(Exception):
    def __init__(self, cause=None) -> None:
        self.cause = cause
