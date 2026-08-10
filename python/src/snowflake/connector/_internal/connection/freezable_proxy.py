"""Freezable proxies for connection state (shared by sync and async implementations).

These proxies serve live values from the core driver while a connection is open,
and fall back to a frozen snapshot after close().
"""

from __future__ import annotations

import threading

from typing import Any

from ..api_client.client_api import core_driver


class _FreezableProxy:
    """Live dict-like proxy; :meth:`freeze` snapshots values for use after close."""

    def __init__(self, conn_handle: Any) -> None:
        self._conn_handle = conn_handle
        self._cache: dict[str, Any] | None = None
        self._freeze_lock = threading.Lock()

    def freeze(self) -> None:
        """Snapshot live values and drop the handle (thread-safe, idempotent)."""
        if self._cache is not None:
            return
        with self._freeze_lock:
            if self._cache is None:
                self._cache = self._fetch_all()
                self._conn_handle = None

    def _fetch_all(self) -> dict[str, Any]:
        raise NotImplementedError


class SessionParametersProxy(_FreezableProxy):
    """Proxy for Snowflake session parameters (case-insensitive keys)."""

    def __getitem__(self, name: str) -> str | None:
        if self._cache is not None:
            return self._cache.get(name.upper())
        response = core_driver.connection_get_parameter(conn_handle=self._conn_handle, key=name)
        return response.value if response.value else None

    def _fetch_all(self) -> dict[str, str]:
        response = core_driver.connection_get_all_parameters(conn_handle=self._conn_handle)
        return {k.upper(): v for k, v in response.parameters.items()}


class ConnectionInfoProxy(_FreezableProxy):
    """Proxy for any field in ConnectionGetInfoResponse.

    Supports all proto fields (role, database, schema, account, warehouse,
    user, host, port, session_id, server_url, session_token, master_token).
    While unfrozen, every field access triggers a connection_get_info RPC.
    """

    def __getitem__(self, field: str) -> Any:
        if self._cache is not None:
            return self._cache.get(field)
        info = core_driver.connection_get_info(conn_handle=self._conn_handle)
        return getattr(info, field) if info.HasField(field) else None  # type: ignore[arg-type]

    def _fetch_all(self) -> dict[str, Any]:
        info = core_driver.connection_get_info(
            conn_handle=self._conn_handle,
            include_master_token=True,
        )
        return {desc.name: value for desc, value in info.ListFields()}
