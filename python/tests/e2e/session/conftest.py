"""Fixtures for e2e session tests.

Three fixtures for inspecting what Python sends to Core across the gRPC boundary:

- ``db_api_mock``: A ``MagicMock`` with minimal stubs for ``Connection.__init__`` to succeed.
  Reusable standalone or composed into ``core_mock``.
- ``core_mock``: Pure mock — no real Core. Use for tests that only verify config passing.
- ``core_proxy``: Wraps the real Core — forwards all calls while recording them.
  Use for tests that need real WireMock behavior AND want to inspect gRPC args.

``core_mock`` and ``core_proxy`` auto-patch ``database_driver_client`` when requested
as a fixture parameter. The driver can be used the same way as without the fixture.
"""

from __future__ import annotations

from typing import Any
from unittest.mock import MagicMock

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    ConnectionIsClosedResponse,
    ConnectionSetOptionsResponse,
    DatabaseHandle,
)


class CoreIntrospector:
    """Inspect what was sent to Core via connection_set_option_* RPCs.

    Works identically for both pure mocks and wrapping proxies because
    ``MagicMock`` records ``.call_args_list`` in both modes.
    """

    def __init__(self, spy: MagicMock) -> None:
        self.db_api = spy

    def get_options_sent(self) -> dict[str, Any]:
        """Extract key->value pairs sent to Core via typed connection_set_option_* RPCs."""
        options: dict[str, Any] = {}
        for call in self.db_api.connection_set_option_bool.call_args_list:
            req = call.args[0]
            options[req.key] = req.value
        for call in self.db_api.connection_set_option_int.call_args_list:
            req = call.args[0]
            options[req.key] = req.value
        for call in self.db_api.connection_set_option_string.call_args_list:
            req = call.args[0]
            options[req.key] = req.value
        return options


@pytest.fixture
def db_api_mock() -> MagicMock:
    """A MagicMock db_api with minimal stubs for Connection.__init__ to work."""
    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    db_api.connection_get_parameter.return_value = MagicMock(value="")
    db_api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=False)
    db_api.connection_set_options.return_value = ConnectionSetOptionsResponse(warnings=[])
    return db_api


@pytest.fixture
def core_mock(monkeypatch: pytest.MonkeyPatch, db_api_mock: MagicMock) -> CoreIntrospector:
    """Pure mock — no real Core. Auto-patches database_driver_client.

    Use for tests that only need to verify what Python passes to Core.
    """
    monkeypatch.setattr("snowflake.connector.connection.database_driver_client", lambda: db_api_mock)
    return CoreIntrospector(db_api_mock)


@pytest.fixture
def core_proxy(monkeypatch: pytest.MonkeyPatch) -> CoreIntrospector:
    """Wraps real Core — forwards all calls while recording gRPC args.

    Use for tests that need real behavior (WireMock) AND want to inspect
    what was passed to Core.
    """
    from snowflake.connector._internal.api_client.client_api import (
        database_driver_client,
    )

    real_client = database_driver_client()
    spy = MagicMock(wraps=real_client)
    monkeypatch.setattr("snowflake.connector.connection.database_driver_client", lambda: spy)
    return CoreIntrospector(spy)
