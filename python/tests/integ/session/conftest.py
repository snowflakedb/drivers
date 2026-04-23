"""Fixtures for integ session tests — Core introspection via pure mock.

Provides ``core_mock`` for tests that verify what Python passes to Core
without requiring a real Core backend or WireMock.
"""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from tests.helpers.core_introspection import CoreIntrospector


@pytest.fixture
def core_mock(monkeypatch: pytest.MonkeyPatch, mock_db_api: MagicMock) -> CoreIntrospector:
    """Pure mock — no real Core. Auto-patches database_driver_client.

    Use for tests that only need to verify what Python passes to Core.
    """
    monkeypatch.setattr("snowflake.connector.connection.database_driver_client", lambda: mock_db_api)
    return CoreIntrospector(mock_db_api)
