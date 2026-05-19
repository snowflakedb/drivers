"""Unit-test conftest.

Overrides fixtures from ``tests/conftest.py`` that would otherwise reach a
real Snowflake backend. Unit tests must remain hermetic — no network, no
``parameters.json`` requirement.
"""

from __future__ import annotations

import pytest


@pytest.fixture
def connection(mock_db_api):
    """Mocked, function-scoped Connection for unit tests.

    Overrides ``tests/conftest.py``'s module-scoped real-connection fixture so
    unit tests don't attempt to talk to a real Snowflake account. ``mock_db_api``
    patches ``core_driver.client`` with a ``MagicMock`` that stubs every RPC
    Connection.__init__ touches.
    """
    from snowflake.connector.connection import Connection

    return Connection(user="test_user", account="test_account")
