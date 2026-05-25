"""Fixtures for integ session tests — Core introspection via pure mock.

Provides ``core_mock`` for tests that verify what Python passes to Core
without requiring a real Core backend or WireMock.
"""

from __future__ import annotations

from unittest.mock import MagicMock

import pytest

from tests.helpers.core_introspection import CoreIntrospector


@pytest.fixture
def core_mock(mock_db_api: MagicMock) -> CoreIntrospector:
    """Pure mock — no real Core. mock_db_api fixture patches core_driver.client.

    Use for tests that only need to verify what Python passes to Core.
    """
    return CoreIntrospector(mock_db_api)
