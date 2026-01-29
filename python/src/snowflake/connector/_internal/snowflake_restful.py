from typing import TYPE_CHECKING

import urllib3.util

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (  # type: ignore[attr-defined]
    ConnectionGetInfoResponse,
)


class SnowflakeRestful:
    """Extend as required"""

    def __init__(self, connection_info: ConnectionGetInfoResponse) -> None:
        self._connection_info = connection_info

    @property
    def token(self) -> str:
        """Required by Python API"""
        return self._connection_info.session_token

    @property
    def _host(self) -> str | None:
        return self._connection_info.host

    @property
    def _protocol(self) -> str | None:
        return urllib3.util.parse_url(self._connection_info.server_url).scheme

    @property
    def _port(self) -> int | None:
        return urllib3.util.parse_url(self._connection_info.server_url).port or 443
