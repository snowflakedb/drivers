from __future__ import annotations

from typing import TYPE_CHECKING
from urllib.parse import urlparse

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConnectionGetInfoResponse


if TYPE_CHECKING:
    from ..connection import SnowflakeConnection


class SnowflakeRestful:
    """Internal only iterface to underlying v1 API"""

    def __init__(self, connection: SnowflakeConnection) -> None:
        self._connection = connection

    @property
    def _connection_info(self) -> ConnectionGetInfoResponse:
        return self._connection._get_connection_info()

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
        # TODO: SNOW-3155971
        raise NotImplementedError()
