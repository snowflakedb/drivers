from __future__ import annotations

import json

from typing import TYPE_CHECKING
from urllib.parse import urlparse

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    TOKEN_REQUEST_TYPE_ISSUE,
    TOKEN_REQUEST_TYPE_RENEW,
    ConnectionGetInfoResponse,
    ConnectionSendHttpRequest,
    ConnectionTokenRequest,
)
from snowflake.connector.errors import OperationalError


if TYPE_CHECKING:
    from ..connection import SnowflakeConnection

DEFAULT_PROTOCOL = "https"
DEFAULT_PORT = 443
APPLICATION_JSON = "application/json"

_TOKEN_REQUEST_TYPE_MAP = {
    "ISSUE": TOKEN_REQUEST_TYPE_ISSUE,
    "RENEW": TOKEN_REQUEST_TYPE_RENEW,
}


class SnowflakeRestful:
    """Backward-compatible REST interface wrapping the driver's internal gRPC API.

    Used by Snowflake CLI to issue Snowflake REST API calls (v1 and v2)
    through the driver's TLS-configured HTTP client.
    """

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
        url = self._connection_info.server_url
        if not url:
            return DEFAULT_PROTOCOL
        return urlparse(url).scheme or DEFAULT_PROTOCOL

    @property
    def _port(self) -> int | None:
        info = self._connection_info
        if info.port:
            return info.port
        url = info.server_url
        if not url:
            return DEFAULT_PORT
        return urlparse(url).port or DEFAULT_PORT

    @property
    def server_url(self) -> str:
        url = self._connection_info.server_url
        if url:
            url = url.rstrip("/")
            parsed = urlparse(url)
            if parsed.port is None:
                port = self._port
                return f"{parsed.scheme}://{parsed.hostname}:{port}{parsed.path}"
            return url
        host = self._host
        if not host:
            raise ValueError("server_url and host are both unavailable from connection info")
        return f"{self._protocol}://{host}:{self._port}"

    @property
    def master_token(self) -> str | None:
        info = self._connection._get_connection_info(include_master_token=True)
        master_token: str = info.master_token
        return master_token if master_token else None

    def _send_http(
        self,
        method: str,
        url: str,
        headers: dict[str, str] | None = None,
        body: dict | str | bytes | None = None,
    ) -> tuple[int, dict[str, str], bytes]:
        if url.startswith(("http://", "https://")):
            parsed = urlparse(url)
            server_parsed = urlparse(self.server_url)
            if parsed.hostname and server_parsed.hostname and parsed.hostname != server_parsed.hostname:
                raise ValueError(
                    f"Absolute URL host '{parsed.hostname}' does not match "
                    f"connection server host '{server_parsed.hostname}'. "
                    f"Pass a relative path instead."
                )
            relative_url = parsed.path
            if parsed.query:
                relative_url = f"{relative_url}?{parsed.query}"
            url = relative_url

        if body is not None:
            if isinstance(body, dict):
                encoded_body = json.dumps(body).encode("utf-8")
            elif isinstance(body, str):
                encoded_body = body.encode("utf-8")
            else:
                encoded_body = body
        else:
            encoded_body = None

        request = ConnectionSendHttpRequest(
            conn_handle=self._connection.conn_handle,
            method=method,
            url=url,
            headers=headers or {},
        )
        if encoded_body is not None:
            request.body = encoded_body

        response = self._connection.db_api.connection_send_http(request)
        return response.status_code, dict(response.headers), response.body

    _SUPPORTED_REQUEST_METHODS = {"get", "post"}

    def request(
        self,
        url: str,
        body: dict | None = None,
        method: str = "post",
        client: str = "sfsql",
        timeout: int | None = None,
        _no_results: bool = False,
        _include_retry_params: bool = False,
        _no_retry: bool = False,
    ) -> dict:
        if client == "sfsql":
            raise NotImplementedError(
                "request() with client='sfsql' is not supported. Use cursor.execute() for SQL queries."
            )

        if method.lower() not in self._SUPPORTED_REQUEST_METHODS:
            raise ValueError(f"Unsupported HTTP method '{method}'. Only 'get' and 'post' are supported.")

        headers = {
            "Content-Type": APPLICATION_JSON,
            "Accept": APPLICATION_JSON,
        }

        status_code, _, response_body = self._send_http(
            method=method,
            url=url,
            headers=headers,
            body=body,
        )

        if status_code >= 400:
            # Truncate body to avoid flooding error messages with large response payloads
            raise OperationalError(msg=f"HTTP {status_code}: {response_body[:200]!r}")

        return json.loads(response_body) if response_body else {}

    def fetch(
        self,
        method: str,
        full_url: str,
        headers: dict[str, str],
        data: str | None = None,
        timeout: int | None = None,
        token: str | None = None,
        no_retry: bool = False,
        raise_raw_http_failure: bool = False,
        **kwargs: object,
    ) -> dict | list:
        status_code, _, response_body = self._send_http(
            method=method,
            url=full_url,
            headers=headers,
            body=data,
        )

        if raise_raw_http_failure and status_code >= 400:
            # Truncate body to avoid flooding error messages with large response payloads
            raise OperationalError(msg=f"HTTP {status_code}: {response_body[:200]!r}")

        return json.loads(response_body) if response_body else {}

    def _token_request(self, request_type: str) -> dict:
        proto_type = _TOKEN_REQUEST_TYPE_MAP.get(request_type.upper())
        if proto_type is None:
            raise ValueError(f"Unknown token request type: {request_type!r}. Must be 'ISSUE' or 'RENEW'.")

        request = ConnectionTokenRequest(
            conn_handle=self._connection.conn_handle,
            request_type=proto_type,
        )
        response = self._connection.db_api.connection_request_token(request)
        data: dict = {"sessionToken": response.session_token}
        if response.HasField("validity_in_seconds"):
            data["validityInSecondsST"] = response.validity_in_seconds
        return {"data": data, "success": True}
