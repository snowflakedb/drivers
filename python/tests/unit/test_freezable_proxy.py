"""Unit tests for the shared session-parameters proxy ``.get()`` accessor.

``get(name, default)`` is the dict-style accessor that legacy
``snowflake-connector-python`` exposes on ``_session_parameters`` (a plain
dict). Snowpark's ``ServerConnection.__init__`` calls it, so the proxy must
provide it. It lives on the shared ``SessionParametersProxyMixin``, so the sync
and async concrete proxies get it from one definition — both are exercised here.
"""

from __future__ import annotations

import pytest

from snowflake.connector.aio.connection._freezable_proxy import (
    _SessionParametersProxy as AsyncSessionParametersProxy,
)
from snowflake.connector.connection._freezable_proxy import SessionParametersProxy


@pytest.fixture(params=[SessionParametersProxy, AsyncSessionParametersProxy], ids=["sync", "async"])
def frozen_proxy(request):
    """A frozen proxy of each concrete type, cache pre-populated (no live core)."""
    proxy = request.param(conn_handle=None)
    proxy._cache = {"AUTOCOMMIT": "true", "TIMEZONE": "UTC"}
    return proxy


class TestSessionParametersProxyGet:
    def test_get_returns_value_case_insensitively(self, frozen_proxy):
        assert frozen_proxy.get("autocommit") == "true"
        assert frozen_proxy.get("AUTOCOMMIT") == "true"

    def test_get_absent_returns_none_by_default(self, frozen_proxy):
        assert frozen_proxy.get("NOT_A_PARAM") is None

    def test_get_absent_returns_supplied_default(self, frozen_proxy):
        assert frozen_proxy.get("NOT_A_PARAM", "fallback") == "fallback"

    def test_get_present_ignores_default(self, frozen_proxy):
        assert frozen_proxy.get("TIMEZONE", "fallback") == "UTC"

    def test_getitem_unaffected(self, frozen_proxy):
        assert frozen_proxy["autocommit"] == "true"
        assert frozen_proxy["NOT_A_PARAM"] is None
