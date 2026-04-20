"""Fixtures for integ session tests — Core introspection via pure mock.

Provides ``core_mock`` for tests that verify what Python passes to Core
without requiring a real Core backend or WireMock.
"""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    ConnectionIsClosedResponse,
    ConnectionSetOptionsResponse,
    DatabaseHandle,
)
from tests.helpers.core_introspection import CoreIntrospector


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
