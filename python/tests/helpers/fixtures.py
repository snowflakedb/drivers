"""Shared pytest fixtures for Core mock and introspection."""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest


@pytest.fixture
def mock_db_api():
    """MagicMock db_api with stubs for all RPCs Connection.__init__ calls."""
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ConnectionHandle,
        ConnectionIsClosedResponse,
        ConnectionSetOptionsResponse,
        DatabaseHandle,
    )

    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    db_api.connection_get_parameter.return_value = MagicMock(value="")
    db_api.connection_is_closed.return_value = ConnectionIsClosedResponse(is_closed=False)
    db_api.connection_set_options.return_value = ConnectionSetOptionsResponse(warnings=[])
    return db_api


@pytest.fixture
def core_proxy(monkeypatch):
    """Wrap real Core client with MagicMock recording. Universal driver only.

    Lazy imports avoid module-level _internal dependency — safe for reference
    connector collection. Tests using this fixture must be marked @skip_reference.
    """
    from snowflake.connector._internal.api_client.client_api import database_driver_client
    from tests.helpers.core_introspection import CoreIntrospector

    real_client = database_driver_client()
    spy = MagicMock(wraps=real_client)
    monkeypatch.setattr("snowflake.connector.connection.database_driver_client", lambda: spy)
    return CoreIntrospector(spy)
