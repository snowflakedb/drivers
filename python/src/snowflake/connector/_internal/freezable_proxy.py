"""Proxies that cache connection state at close time for post-close access."""

from __future__ import annotations

import threading

from typing import Any

from .protobuf_gen.database_driver_v1_pb2 import (
    ConnectionGetAllParametersRequest,
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionGetParameterRequest,
)


class FreezableProxy:
    """Dict-like proxy that fetches live values while open and serves cached values after freeze().

    Subclasses implement _fetch_one (single key) and _fetch_all (bulk snapshot).
    Call freeze() before releasing the underlying handles.
    """

    def __init__(self, db_api: Any, conn_handle: Any) -> None:
        self._db_api = db_api
        self._conn_handle = conn_handle
        self._cache: dict[str, Any] | None = None
        self._freeze_lock = threading.Lock()

    def freeze(self) -> None:
        """Take a snapshot and release references to db_api/conn_handle.

        Thread-safe: concurrent close() calls race through here; double-checked
        locking ensures only the first caller fetches and the rest no-op.
        """
        if self._cache is not None:
            return
        with self._freeze_lock:
            if self._cache is None:
                self._cache = self._fetch_all()
                self._db_api = None
                self._conn_handle = None

    def _fetch_one(self, key: str) -> Any:
        raise NotImplementedError

    def _fetch_all(self) -> dict[str, Any]:
        raise NotImplementedError

    def __getitem__(self, key: str) -> Any:
        if self._cache is not None:
            return self._cache.get(key)
        return self._fetch_one(key)


class SessionParametersProxy(FreezableProxy):
    """Proxy for Snowflake session parameters (case-insensitive keys)."""

    def _fetch_one(self, name: str) -> str | None:
        request = ConnectionGetParameterRequest(conn_handle=self._conn_handle, key=name)
        response = self._db_api.connection_get_parameter(request)
        return response.value if response.value else None

    def _fetch_all(self) -> dict[str, str]:
        request = ConnectionGetAllParametersRequest(conn_handle=self._conn_handle)
        response = self._db_api.connection_get_all_parameters(request)
        return {k.upper(): v for k, v in response.parameters.items()}

    def __getitem__(self, name: str) -> str | None:
        if self._cache is not None:
            return self._cache.get(name.upper())
        return self._fetch_one(name)


class ConnectionInfoProxy(FreezableProxy):
    """Proxy for any field in ConnectionGetInfoResponse.

    Supports all proto fields (role, database, schema, account, warehouse,
    user, host, port, session_id, server_url, session_token, master_token).
    While unfrozen, every field access triggers a connection_get_info RPC.
    This matches the pre-proxy behavior where each property did its own RPC.
    """

    def _fetch_info(self, include_master_token: bool = False) -> ConnectionGetInfoResponse:
        request = ConnectionGetInfoRequest(
            conn_handle=self._conn_handle,
            include_master_token=include_master_token,
        )
        result: ConnectionGetInfoResponse = self._db_api.connection_get_info(request)
        return result

    def _fetch_one(self, field: str) -> Any:
        info = self._fetch_info()
        return getattr(info, field) if info.HasField(field) else None  # type: ignore[arg-type]

    def _fetch_all(self) -> dict[str, Any]:
        info = self._fetch_info(include_master_token=True)
        return {desc.name: value for desc, value in info.ListFields()}
