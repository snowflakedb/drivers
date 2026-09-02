"""Freezable proxies for connection state (shared by sync and async implementations).

These proxies serve live values from the core driver while a connection is open,
and fall back to a frozen snapshot after close().
"""

from __future__ import annotations

import threading

from typing import TYPE_CHECKING, Any

from ..api_client.client_api import core_driver
from ..decorators import snowpark_compat


if TYPE_CHECKING:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConfigSetting


SessionParameterValue = str | bool | int | float


def _config_setting_to_python(setting: ConfigSetting) -> SessionParameterValue | None:
    """Read whichever oneof variant is set on a ConfigSetting as a native Python value."""
    which = setting.WhichOneof("value")
    return None if which is None else getattr(setting, which)


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

    def __getitem__(self, name: str) -> SessionParameterValue | None:
        if self._cache is not None:
            return self._cache.get(name.upper())
        return self._fetch_one(name)

    @snowpark_compat
    def get(self, name: str, default: SessionParameterValue | None = None) -> SessionParameterValue | None:
        """Dict-style lookup returning ``default`` when the parameter is unset.

        Legacy ``snowflake-connector-python`` stores ``_session_parameters`` as a
        plain dict, so callers (e.g. Snowpark's ``ServerConnection``) use
        ``.get(name, default)``. A populated session parameter is always a
        non-``None`` value, so a ``None`` result means "unset" here.
        """
        value = self[name]
        return value if value is not None else default

    def _fetch_one(self, name: str) -> SessionParameterValue | None:
        response = core_driver.connection_get_parameter(conn_handle=self._conn_handle, key=name)
        return _config_setting_to_python(response.typed_value) if response.HasField("typed_value") else None

    def _get_string(self, name: str) -> str | None:
        value = self[name]
        return value if isinstance(value, str) else None

    def _fetch_all(self) -> dict[str, SessionParameterValue]:
        response = core_driver.connection_get_all_parameters(conn_handle=self._conn_handle)
        values = ((key.upper(), _config_setting_to_python(value)) for key, value in response.typed_parameters.items())
        return {key: value for key, value in values if value is not None}


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
