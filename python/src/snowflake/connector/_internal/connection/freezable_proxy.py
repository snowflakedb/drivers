"""Shared freezable-proxy access logic for sync and async connection implementations."""

from __future__ import annotations

from typing import Any

from ..api_client.client_api import core_driver


class SessionParametersProxyMixin:
    """Shared ``__getitem__`` / ``_fetch_one`` for session-parameter proxies."""

    _conn_handle: Any
    _cache: dict[str, Any] | None

    def __getitem__(self, name: str) -> str | None:
        if self._cache is not None:
            return self._cache.get(name.upper())
        return self._fetch_one(name)

    def _fetch_one(self, name: str) -> str | None:
        response = core_driver.connection_get_parameter(conn_handle=self._conn_handle, key=name)
        return response.value if response.value else None


class ConnectionInfoProxyMixin:
    """Shared ``__getitem__`` / ``_fetch_one`` for connection-info proxies."""

    _conn_handle: Any
    _cache: dict[str, Any] | None

    def __getitem__(self, field: str) -> Any:
        if self._cache is not None:
            return self._cache.get(field)
        return self._fetch_one(field)

    def _fetch_one(self, field: str) -> Any:
        info = core_driver.connection_get_info(conn_handle=self._conn_handle)
        return getattr(info, field) if info.HasField(field) else None  # type: ignore[arg-type]
