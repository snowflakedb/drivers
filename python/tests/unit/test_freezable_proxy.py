"""Unit tests for the shared session-parameters proxy ``.get()`` accessor.

``get(name, default)`` is the dict-style accessor that legacy
``snowflake-connector-python`` exposes on ``_session_parameters`` (a plain
dict). Snowpark's ``ServerConnection.__init__`` calls it, so the proxy must
provide it. ``SessionParametersProxy`` is shared verbatim by the sync and
async connection implementations (one definition, no separate subclasses).
"""

from __future__ import annotations

import pytest

from snowflake.connector._internal.connection.freezable_proxy import SessionParametersProxy


@pytest.fixture
def frozen_proxy():
    """A frozen proxy, cache pre-populated (no live core)."""
    proxy = SessionParametersProxy(conn_handle=None)
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
