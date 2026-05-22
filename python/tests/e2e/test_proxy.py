"""End-to-end proxy support tests using Wiremock as a forward HTTP proxy.

Tests live in ``tests/e2e`` so they execute against both the universal driver
(``dev`` env) and the reference connector (``reference`` env) — the precedence
test in particular demonstrates the deliberate behaviour difference recorded
in ``BehaviorDifferences.yaml`` entry 30.

Wiremock is started with ``--enable-browser-proxying --proxy-pass-through=false``
in ``WiremockClient``, so it acts as a forward HTTP proxy that matches incoming
requests by URL path against its registered mappings. Setting ``server_url`` to
a hostname that cannot be resolved (``nonexistent.invalid``) gives us an
unambiguous signal: a successful login means the request transited the proxy;
a connection failure means the driver attempted direct DNS resolution.
"""

from __future__ import annotations

from urllib.parse import urlparse

import pytest

from tests.compatibility import IS_UNIVERSAL_DRIVER
from tests.private_key_helper import get_test_private_key_path
from tests.wiremock_client import WiremockClient


_UNRESOLVABLE_HOST = "nonexistent.invalid"
_UNRESOLVABLE_SERVER_URL = f"http://{_UNRESOLVABLE_HOST}:8090"


def _build_connection(connector_adapter, **overrides):
    """Build a connection pointing at an unresolvable host so only proxy-routed
    requests can succeed."""
    from tests.connector_factory import create_connection_with_adapter

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
    }
    params.update(overrides)
    return create_connection_with_adapter(connector_adapter, **params)


def _login_requests(wiremock: WiremockClient) -> list[dict]:
    return [
        r
        for r in wiremock.get_all_requests()
        if "/session/v1/login-request" in r.get("request", {}).get("url", "")
    ]


class TestProxyConnectionParams:
    """Connection-parameter-driven proxy routing."""

    def test_proxy_host_routes_request_through_proxy(self, connector_adapter):
        """Driver configured with proxy_host/port should route login through
        the proxy. Legacy snowflake-connector-python kwargs."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            conn = _build_connection(
                connector_adapter,
                proxy_host="localhost",
                proxy_port=wiremock.http_port,
            )

            assert len(_login_requests(wiremock)) >= 1, (
                "Login request should have been routed through the proxy"
            )
            conn.close(retry=False)

    @pytest.mark.skip_reference(
        reason="Legacy ODBC PROXY URL form is universal-driver-only; reference connector "
        "uses proxy_host/proxy_port instead"
    )
    def test_legacy_odbc_proxy_url_routes_request_through_proxy(self, connector_adapter):
        """Legacy ODBC ``PROXY=http://host:port`` URL form is parsed by sf_core
        and merged with individual fields."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            conn = _build_connection(
                connector_adapter,
                proxy=f"http://localhost:{wiremock.http_port}",
            )

            assert len(_login_requests(wiremock)) >= 1, (
                "Login should have been routed through the proxy URL"
            )
            conn.close(retry=False)

    @pytest.mark.skip_reference(
        reason="Reference connector ignores no_proxy when no proxy host is configured; "
        "test asserts new-driver semantics"
    )
    def test_no_proxy_bypasses_proxy_for_matching_host(self, connector_adapter):
        """``no_proxy`` matching the target host should bypass the proxy and let
        direct resolution fail."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            with pytest.raises(Exception):  # noqa: PT011 — driver-specific class varies
                _build_connection(
                    connector_adapter,
                    proxy_host="localhost",
                    proxy_port=wiremock.http_port,
                    no_proxy=_UNRESOLVABLE_HOST,
                )

            assert len(_login_requests(wiremock)) == 0, (
                "no_proxy bypass should prevent the proxy from receiving the login"
            )


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
    def test_http_proxy_env_var_routes_request_when_use_proxy_env_true(
        self, connector_adapter, monkeypatch
    ):
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            monkeypatch.setenv("HTTP_PROXY", wiremock.http_url())

            conn = _build_connection(connector_adapter, use_proxy_env=True)

            # WiremockClient admin queries use the requests library, which
            # also honours HTTP_PROXY. Unset before asserting so the admin
            # query reaches wiremock directly.
            monkeypatch.delenv("HTTP_PROXY")

            assert len(_login_requests(wiremock)) >= 1, (
                "use_proxy_env=True with HTTP_PROXY env var should route through proxy"
            )
            conn.close(retry=False)

    @pytest.mark.skip_reference(
        reason="Reference connector always reads env vars; default-deny is universal-only"
    )
    def test_http_proxy_env_var_ignored_by_default(self, connector_adapter, monkeypatch):
        """Default ``use_proxy_env=False``: env vars are NOT consulted, so the
        login attempt fails on direct DNS resolution rather than transiting
        the proxy."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            monkeypatch.setenv("HTTP_PROXY", wiremock.http_url())

            with pytest.raises(Exception):  # noqa: PT011 — driver-specific class varies
                _build_connection(connector_adapter)

            monkeypatch.delenv("HTTP_PROXY")

            assert len(_login_requests(wiremock)) == 0, (
                "Default-deny: HTTP_PROXY env var must not be picked up without use_proxy_env=True"
            )


class TestProxyPrecedence:
    """Connection params vs env vars precedence.

    Cross-version test that locks in ``BehaviorDifferences.yaml`` entry 30:

    - Legacy connector ≥3.17.0: ``HTTP_PROXY`` env var overrides explicit
      ``proxy_host`` connection parameters.
    - Universal driver: explicit ``proxy_host`` always wins. Env vars are only
      consulted as a fallback when no explicit param is set, AND the customer
      has opted in via ``use_proxy_env=True``.
    """

    def test_explicit_proxy_param_takes_precedence_over_env_var(
        self, connector_adapter, monkeypatch
    ):
        """When both are set, only the explicit proxy_host should receive
        requests on the universal driver. The legacy branch documents the
        inverted behaviour rather than skipping."""
        with (
            WiremockClient().start() as proxy_via_param,
            WiremockClient().start() as proxy_via_env,
        ):
            proxy_via_param.add_mapping("auth/login_success_jwt.json")
            proxy_via_env.add_mapping("auth/login_success_jwt.json")

            monkeypatch.setenv("HTTP_PROXY", proxy_via_env.http_url())

            conn = _build_connection(
                connector_adapter,
                proxy_host="localhost",
                proxy_port=proxy_via_param.http_port,
                # Even with use_proxy_env=True, the explicit param must win.
                use_proxy_env=True,
            )

            # WiremockClient admin queries use the requests library, which
            # also honours HTTP_PROXY. Unset before asserting.
            monkeypatch.delenv("HTTP_PROXY")

            param_hits = len(_login_requests(proxy_via_param))
            env_hits = len(_login_requests(proxy_via_env))

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

            conn.close(retry=False)
