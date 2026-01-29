from __future__ import annotations

import urllib3.util

from snowflake.connector._internal.protobuf_gen.database_driver_v1_services import (  # type: ignore[attr-defined]
    ConnectionGetInfoResponse,
)


class SnowflakeRestful:
    """Extend as required"""

    def __init__(self, connection_info: ConnectionGetInfoResponse) -> None:
        self._connection_info = connection_info

    @property
    def token(self) -> str | None:
        """Required by Python API"""
        return self._connection_info.session_token  # type: ignore

    @property
    def _host(self) -> str | None:
        return self._connection_info.host  # type: ignore

    @property
    def _protocol(self) -> str | None:
        if self._connection_info.server_url is None:
            return None
        return urllib3.util.parse_url(self._connection_info.server_url).scheme

    @property
    def _port(self) -> int | None:
        if self._connection_info.server_url is None:
            return None
        return urllib3.util.parse_url(self._connection_info.server_url).port or 443

    @property
    def master_token(self) -> str | None:
        return "TODO"
