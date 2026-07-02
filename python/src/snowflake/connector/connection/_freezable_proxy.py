"""Sync proxies that cache connection state at close time for post-close access."""

from __future__ import annotations

import threading

from typing import Any

from .._internal.api_client.client_api import core_driver
from .._internal.connection.freezable_proxy import ConnectionInfoProxyMixin, SessionParametersProxyMixin


# ------------------------------------------------------------------
# Base proxy
# ------------------------------------------------------------------


class FreezableProxy:
    """Dict-like proxy that fetches live values while open and serves cached values after freeze().

    Subclasses implement _fetch_all (bulk snapshot).  Item access is provided by the
    companion mixin (:class:`SessionParametersProxyMixin` or
    :class:`ConnectionInfoProxyMixin`).
    Call freeze() before releasing the underlying handles.
    """

    def __init__(self, conn_handle: Any) -> None:
        self._conn_handle = conn_handle
        self._cache: dict[str, Any] | None = None
        self._freeze_lock = threading.Lock()

    def freeze(self) -> None:
        """Take a snapshot and release references to conn_handle.

        Thread-safe: concurrent close() calls race through here; double-checked
        locking ensures only the first caller fetches and the rest no-op.
        """
        if self._cache is not None:
            return
        with self._freeze_lock:
            if self._cache is None:
                self._cache = self._fetch_all()
                self._conn_handle = None

    def _fetch_all(self) -> dict[str, Any]:
        raise NotImplementedError


# ------------------------------------------------------------------
# Session parameters
# ------------------------------------------------------------------


class SessionParametersProxy(FreezableProxy, SessionParametersProxyMixin):
    """Proxy for Snowflake session parameters (case-insensitive keys)."""

    def _fetch_all(self) -> dict[str, str]:
        response = core_driver.connection_get_all_parameters(conn_handle=self._conn_handle)
        return {k.upper(): v for k, v in response.parameters.items()}


# ------------------------------------------------------------------
# Connection info
# ------------------------------------------------------------------


class ConnectionInfoProxy(FreezableProxy, ConnectionInfoProxyMixin):
    """Proxy for any field in ConnectionGetInfoResponse.

    Supports all proto fields (role, database, schema, account, warehouse,
    user, host, port, session_id, server_url, session_token, master_token).
    While unfrozen, every field access triggers a connection_get_info RPC.
    This matches the pre-proxy behavior where each property did its own RPC.
    """

    def _fetch_all(self) -> dict[str, Any]:
        info = core_driver.connection_get_info(
            conn_handle=self._conn_handle,
            include_master_token=True,
        )
        return {desc.name: value for desc, value in info.ListFields()}
