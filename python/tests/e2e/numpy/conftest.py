"""Shared fixtures for numpy type tests."""

from __future__ import annotations

import pytest

from tests.conftest import _wrap_for_async_backends
from tests.connector_factory import create_connection_with_adapter


@pytest.fixture(scope="module")
def connection(request, connector_adapter, connection_backend, cursor_backend):
    """Module-scoped connection with ``numpy=True``."""
    paramstyle = getattr(request, "param", None)
    conn = create_connection_with_adapter(connector_adapter, paramstyle=paramstyle, numpy=True)
    wrapped = _wrap_for_async_backends(
        conn,
        connection_backend=connection_backend,
        cursor_backend=cursor_backend,
    )
    try:
        yield wrapped
    finally:
        if not wrapped.is_closed():
            wrapped.close()
