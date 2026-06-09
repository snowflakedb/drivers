"""End-to-end proxy support tests using Wiremock as a forward HTTP proxy.

Tests live in ``tests/e2e/session`` and execute against both the universal
driver (``dev`` env) and the reference connector (``reference`` env) — the
precedence test in particular demonstrates the deliberate behaviour difference
recorded
in ``BehaviorDifferences.yaml`` entry 31.

The shared ``wiremock`` fixture runs Wiremock with ``--enable-browser-proxying
--proxy-pass-through=false``, so it acts as a forward HTTP proxy that matches
incoming requests by URL path against registered mappings. Setting
``server_url`` to a hostname that cannot be resolved (``nonexistent.invalid``)
gives us an unambiguous signal: a successful login means the request transited
the proxy; a connection failure means the driver attempted direct DNS resolution.
"""

from __future__ import annotations

from urllib.parse import urlparse

import pytest

from tests.compatibility import IS_UNIVERSAL_DRIVER
from tests.connector_factory import create_connection_with_adapter
from tests.private_key_helper import get_test_private_key_path
from tests.wiremock_client import WiremockClient


_UNRESOLVABLE_HOST = "nonexistent.invalid"
_UNRESOLVABLE_SERVER_URL = f"http://{_UNRESOLVABLE_HOST}:8090"


def _build_connection(connector_adapter, **overrides):
    """Build a connection pointing at an unresolvable host so only proxy-routed
    requests can succeed."""
    parsed = urlparse(_UNRESOLVABLE_SERVER_URL)
    params = {
        "account": "test_account",
        "user": "test_user",
        "database": "test_database",
        "schema": "test_schema",
        "warehouse": "test_warehouse",
        "role": "test_role",
        "server_url": _UNRESOLVABLE_SERVER_URL,
        "protocol": parsed.scheme,
        "host": parsed.hostname,
        "port": parsed.port,
        "authenticator": "SNOWFLAKE_JWT",
        "private_key_file": get_test_private_key_path(),
        # Match other Wiremock session tests: avoid async-query probing on close.
        "enable_server_session_keep_alive_auto_detection": False,
    }
    params.update(overrides)
    return create_connection_with_adapter(connector_adapter, **params)


def _login_requests(wiremock: WiremockClient) -> list[dict]:
    return [
        r for r in wiremock.get_all_requests() if "/session/v1/login-request" in r.get("request", {}).get("url", "")
    ]


class TestProxyConnectionParams:
    """Connection-parameter-driven proxy routing."""

    def test_should_route_request_through_proxy_when_proxy_host_is_configured(self, connector_adapter, wiremock):
        """Driver configured with proxy_host/port should route login through
        the proxy. Legacy snowflake-connector-python kwargs."""
        # Given a forward-proxy WireMock serving a canned login response
        wiremock.add_mapping("auth/login_success_jwt.json")

        # When the driver connects with proxy_host and proxy_port pointing at the proxy
        conn = _build_connection(
            connector_adapter,
            proxy_host="localhost",
            proxy_port=wiremock.http_port,
        )
        try:
            # Then the proxy received the login request
            assert len(_login_requests(wiremock)) >= 1, "Login request should have been routed through the proxy"
        finally:
            conn.close(retry=False)

    @pytest.mark.skip_reference(
        reason="Legacy ODBC PROXY URL form is universal-driver-only; reference connector "
        "uses proxy_host/proxy_port instead"
    )
    def test_should_route_login_through_proxy_using_legacy_odbc_proxy_url(self, connector_adapter, wiremock):
        """Legacy ODBC ``PROXY=http://host:port`` URL form is parsed by sf_core
        and merged with individual fields."""
        # Given a forward-proxy WireMock serving a canned login response
        wiremock.add_mapping("auth/login_success_jwt.json")

        # When the driver connects with PROXY pointing at the proxy
        conn = _build_connection(
            connector_adapter,
            proxy=f"http://localhost:{wiremock.http_port}",
        )
        try:
            # Then the proxy received the login request
            assert len(_login_requests(wiremock)) >= 1, "Login should have been routed through the proxy URL"
        finally:
            conn.close(retry=False)

    @pytest.mark.skip_reference(
        reason="Reference connector ignores no_proxy when no proxy host is configured; "
        "test asserts new-driver semantics"
    )
    def test_should_bypass_proxy_when_no_proxy_matches_the_target_host(self, connector_adapter, wiremock):
        """``no_proxy`` matching the target host should bypass the proxy and let
        direct resolution fail."""
        # Given a forward-proxy WireMock serving a canned login response
        wiremock.add_mapping("auth/login_success_jwt.json")

        # When the driver connects with proxy_host and no_proxy matching the target
        with pytest.raises(Exception):  # noqa: B017 — driver-specific class varies
            _build_connection(
                connector_adapter,
                proxy_host="localhost",
                proxy_port=wiremock.http_port,
                no_proxy=_UNRESOLVABLE_HOST,
            )

        # Then the connect fails and the proxy received no requests
        assert len(_login_requests(wiremock)) == 0, "no_proxy bypass should prevent the proxy from receiving the login"


class TestProxyEnvVars:
    """Environment-variable-driven proxy routing.

    Universal driver: ``HTTP_PROXY``/``HTTPS_PROXY``/``NO_PROXY`` are ignored
    by default; ``use_proxy_env=True`` opts in to env detection. Legacy
    connector: env vars are always honoured. The precedence test below makes
    this divergence explicit.
    """

    @pytest.mark.skip_reference(
        reason="use_proxy_env is a universal-driver-only opt-in; reference connector always reads env vars"
    )
    def test_should_route_request_through_proxy_when_use_proxy_env_is_true(
        self, connector_adapter, wiremock, monkeypatch
    ):
        # Given HTTP_PROXY env var points at a forward-proxy WireMock
        wiremock.add_mapping("auth/login_success_jwt.json")

        monkeypatch.setenv("HTTP_PROXY", wiremock.http_url())

        # When the driver connects with use_proxy_env=True
        conn = _build_connection(connector_adapter, use_proxy_env=True)
        try:
            # WiremockClient admin queries use the requests library, which
            # also honours HTTP_PROXY. Unset before asserting so the admin
            # query reaches wiremock directly.
            monkeypatch.delenv("HTTP_PROXY")

            # Then the proxy received the login request
            assert len(_login_requests(wiremock)) >= 1, (
                "use_proxy_env=True with HTTP_PROXY env var should route through proxy"
            )
        finally:
            conn.close(retry=False)

    @pytest.mark.skip_reference(reason="Reference connector always reads env vars; default-deny is universal-only")
    def test_should_ignore_http_proxy_env_var_by_default(self, connector_adapter, wiremock, monkeypatch):
        """Default ``use_proxy_env=False``: env vars are NOT consulted, so the
        login attempt fails on direct DNS resolution rather than transiting
        the proxy."""
        # Given HTTP_PROXY env var points at a forward-proxy WireMock
        wiremock.add_mapping("auth/login_success_jwt.json")

        monkeypatch.setenv("HTTP_PROXY", wiremock.http_url())

        # When the driver connects without use_proxy_env
        with pytest.raises(Exception):  # noqa: B017 — driver-specific class varies
            _build_connection(connector_adapter)

        monkeypatch.delenv("HTTP_PROXY")

        # Then the connect fails and the proxy received no requests
        assert len(_login_requests(wiremock)) == 0, (
            "Default-deny: HTTP_PROXY env var must not be picked up without use_proxy_env=True"
        )


class TestProxyPrecedence:
    """Connection params vs env vars precedence.

    Cross-version test that locks in ``BehaviorDifferences.yaml`` entry 31:

    - Legacy connector ≥3.17.0: ``HTTP_PROXY`` env var overrides explicit
      ``proxy_host`` connection parameters.
    - Universal driver: explicit ``proxy_host`` always wins. Env vars are only
      consulted as a fallback when no explicit param is set, AND the customer
      has opted in via ``use_proxy_env=True``.
    """

    def test_should_prefer_explicit_proxy_host_over_http_proxy_env_var(self, connector_adapter, wiremock, monkeypatch):
        """When both are set, only the explicit proxy_host should receive
        requests on the universal driver. The legacy branch documents the
        inverted behaviour rather than skipping."""
        # Given two forward-proxy WireMock instances are running
        proxy_via_param = wiremock
        proxy_via_param.add_mapping("auth/login_success_jwt.json")

        # Second JVM required: one Wiremock process binds a single proxy port.
        with WiremockClient().start() as proxy_via_env:
            proxy_via_env.add_mapping("auth/login_success_jwt.json")

            # And HTTP_PROXY env var points at the second proxy
            monkeypatch.setenv("HTTP_PROXY", proxy_via_env.http_url())

            # When the driver connects with proxy_host pointing at the first proxy
            conn = _build_connection(
                connector_adapter,
                proxy_host="localhost",
                proxy_port=proxy_via_param.http_port,
                # Even with use_proxy_env=True, the explicit param must win.
                use_proxy_env=True,
            )
            try:
                # WiremockClient admin queries use the requests library, which
                # also honours HTTP_PROXY. Unset before asserting.
                monkeypatch.delenv("HTTP_PROXY")

                param_hits = len(_login_requests(proxy_via_param))
                env_hits = len(_login_requests(proxy_via_env))

                # Then only the first proxy received the login request
                if IS_UNIVERSAL_DRIVER:
                    assert param_hits >= 1 and env_hits == 0, (
                        "Universal driver: explicit proxy_host must override HTTP_PROXY env var "
                        f"(param hits={param_hits}, env hits={env_hits})"
                    )
                else:
                    # Legacy connector ≥3.17.0 inverts precedence: env var wins.
                    assert env_hits >= 1 and param_hits == 0, (
                        "Legacy connector: HTTP_PROXY env var overrides proxy_host param "
                        f"(param hits={param_hits}, env hits={env_hits})"
                    )
            finally:
                conn.close(retry=False)
