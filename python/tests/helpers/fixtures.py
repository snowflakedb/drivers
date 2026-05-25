"""Shared pytest fixtures for Core mock and introspection."""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest


@pytest.fixture
def mock_db_api():
    """MagicMock db_api patched into core_driver for Connection.__init__ tests.

    Sets ``core_driver.client`` to the mock so that all RPC calls flow
    through the mock. Restores the previous client on teardown.
    """
    from snowflake.connector._internal.api_client.client_api import core_driver
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

    old_client = core_driver._client
    core_driver.client = db_api
    yield db_api
    core_driver.client = old_client


@pytest.fixture
def core_proxy():
    """Wrap real Core client with MagicMock recording. Universal driver only.

    Lazy imports avoid module-level _internal dependency — safe for reference
    connector collection. Tests using this fixture must be marked @skip_reference.
    """
    from snowflake.connector._internal.api_client.client_api import core_driver
    from tests.helpers.core_introspection import CoreIntrospector

    real_client = core_driver.client
    spy = MagicMock(wraps=real_client)
    core_driver.client = spy
    yield CoreIntrospector(spy)
    core_driver.client = real_client
