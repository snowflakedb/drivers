from __future__ import annotations

from urllib.parse import urlparse

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (
    ConnectionGetInfoResponse,
)


class SnowflakeRestful:
    """Extend as required"""

    def __init__(self, connection_info: ConnectionGetInfoResponse) -> None:
        self._connection_info = connection_info

    @property
    def token(self) -> str | None:
        """Required by Python API"""
        session_token: str | None = self._connection_info.session_token
        return session_token

    @property
    def _host(self) -> str | None:
        host: str | None = self._connection_info.host
        return host

    @property
    def _protocol(self) -> str | None:
        return urlparse(self._connection_info.server_url).scheme

    @property
    def _port(self) -> int | None:
        return urlparse(self._connection_info.server_url).port or 443

    @property
    def master_token(self) -> str | None:
        return "TODO"
