"""End-to-end OAuth tests for the Python wrapper.

These tests drive a real Snowflake account / IdP through
``snowflake.connector.connect()``. They are gated behind the
``SNOWFLAKE_TEST_OAUTH_*`` parameters in ``parameters.json`` and
``pytest.skip`` when the relevant fields are missing -- mirroring the
``test_pat.py`` / ``test_user_password_mfa.py`` patterns and the
``oauth.cpp`` E2E gating in ``odbc_tests/tests/e2e/authentication/`` and
the ``oauth.rs`` E2E gating in ``sf_core/tests/e2e/authentication/``.

The Authorization Code happy-path scenarios spawn the OS browser via
sf_core's loopback listener; they are gated additionally behind
``SNOWFLAKE_OAUTH_E2E_BROWSER=1`` so a developer can opt in. The Client
Credentials and legacy ``AUTHENTICATOR=OAUTH`` paths do not require a
browser and run whenever the matching parameters are configured.

Required ``parameters.json`` keys (cross-driver configuration matrix):

* ``SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN``       legacy OAUTH / AC short-circuit
* ``SNOWFLAKE_TEST_OAUTH_CLIENT_ID``          AC + CC
* ``SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET``      AC + CC
* ``SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL``  AC (optional)
* ``SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL``  AC (optional) / CC (required)
* ``SNOWFLAKE_TEST_OAUTH_REDIRECT_URI``       AC (optional)
* ``SNOWFLAKE_TEST_OAUTH_SCOPE``              AC + CC (optional)

Test method names mirror sf_core's existing ``oauth_should_*`` methods
in ``sf_core/tests/e2e/authentication/oauth.rs`` (and the ODBC
equivalents) so the same Gherkin scenarios in
``tests/definitions/shared/authentication/oauth.feature`` validate
against every implementation. Scenario step text below is taken
verbatim from the corresponding sf_core / ODBC comments.
"""

from __future__ import annotations

import os

import pytest

from ...config import get_test_parameters
from .auth_helpers import verify_simple_query_execution


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _require_oauth_param(params: dict[str, object], key: str) -> str:
    """Return ``params[key]`` or skip the test when it's missing / empty."""
    value = params.get(key)
    if not isinstance(value, str) or not value:
        pytest.skip(f"OAuth E2E test requires {key} in parameters.json")
    return value


def _add_oauth_optional(kwargs: dict[str, object], params: dict[str, object], cfg_key: str, conn_key: str) -> None:
    """Set ``kwargs[conn_key]`` from ``params[cfg_key]`` when the parameter is provided."""
    value = params.get(cfg_key)
    if isinstance(value, str) and value:
        kwargs[conn_key] = value


def _require_oauth_browser_opt_in(message: str) -> None:
    """Skip the test unless ``SNOWFLAKE_OAUTH_E2E_BROWSER=1`` is set.

    The OAuth Authorization Code happy-path scenarios spawn the OS
    browser via sf_core's loopback listener; we gate them so they do
    not run by accident in CI.
    """
    if os.environ.get("SNOWFLAKE_OAUTH_E2E_BROWSER") != "1":
        pytest.skip(f"OAuth AC E2E spawns a real OS browser; opt in with SNOWFLAKE_OAUTH_E2E_BROWSER=1: {message}")


@pytest.fixture(scope="module")
def oauth_params() -> dict[str, object]:
    """Expose the full parameters.json for OAuth-specific lookups."""
    return get_test_parameters()


# ---------------------------------------------------------------------------
# Legacy AUTHENTICATOR=OAUTH (pre-acquired access token)
# ---------------------------------------------------------------------------


class TestLegacyOAuthAccessToken:
    def test_oauth_should_authenticate_with_pre_acquired_access_token(self, connection_factory, oauth_params):
        # Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=`
        access_token = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN")
        authenticator = "OAUTH"

        # When Trying to Connect
        connection = connection_factory(authenticator=authenticator, token=access_token)

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_oauth_should_fail_legacy_authentication_with_invalid_token(self, connection_factory):
        # Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
        authenticator = "OAUTH"
        invalid_token = "invalid_oauth_token_12345"

        # When Trying to Connect
        with pytest.raises(Exception) as exc_info:
            connection_factory(authenticator=authenticator, token=invalid_token)

        # Then Connection fails with an authentication / login error
        from snowflake.connector.errors import DatabaseError

        assert isinstance(exc_info.value, DatabaseError), f"Expected DatabaseError, got: {type(exc_info.value)}"

    def test_oauth_should_authenticate_using_lowercase_oauth_authenticator(self, connection_factory, oauth_params):
        # Given Authentication is set to lowercase oauth and a valid pre-acquired
        #       OAuth access token is supplied via TOKEN
        access_token = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_ACCESS_TOKEN")
        authenticator = "oauth"

        # When Trying to Connect
        connection = connection_factory(authenticator=authenticator, token=access_token)

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)


# ---------------------------------------------------------------------------
# OAuth Authorization Code (AC) flow
# ---------------------------------------------------------------------------


class TestOAuthAuthorizationCode:
    """The AC flow requires a real browser leg unless an access token is pre-seeded.

    The keyring-short-circuit scenario lives in sf_core's Rust e2e
    suite (the seeding helper is not exposed to Python). The two
    scenarios here therefore opt in via ``SNOWFLAKE_OAUTH_E2E_BROWSER=1``.
    """

    def test_oauth_should_authenticate_using_authorization_code_flow(self, connection_factory, oauth_params):
        _require_oauth_browser_opt_in("Authorization Code happy path")
        client_id = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID")
        client_secret = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET")

        # Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret.
        #       `oauth_authorization_url` and `oauth_token_request_url` are forwarded from parameters
        #       when present (otherwise the driver falls back to the Snowflake-IdP defaults
        #       `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request`).
        #       `client_store_temporary_credential=true` lets the AC flow short-circuit on subsequent
        #       runs by re-using the cached access / refresh token (AC state machine: cache → refresh → interactive).
        kwargs: dict[str, object] = {
            "authenticator": "OAUTH_AUTHORIZATION_CODE",
            "oauth_client_id": client_id,
            "oauth_client_secret": client_secret,
            "client_store_temporary_credential": True,
        }
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL", "oauth_authorization_url")
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL", "oauth_token_request_url")
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_REDIRECT_URI", "oauth_redirect_uri")
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "oauth_scope")

        # When Trying to Connect (this will spawn the local-loopback HTTP listener and
        #      `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached
        #      access token short-circuits the leg)
        connection = connection_factory(**kwargs)

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_oauth_should_fail_authorization_code_flow_with_bad_client_secret(self, connection_factory, oauth_params):
        _require_oauth_browser_opt_in("Authorization Code negative path (browser leg still required)")
        client_id = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID")

        # Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a
        #       deliberately invalid client secret. The IdP token-exchange step must reject the
        #       credentials and the driver must surface an authentication / login error.
        kwargs: dict[str, object] = {
            "authenticator": "OAUTH_AUTHORIZATION_CODE",
            "oauth_client_id": client_id,
            "oauth_client_secret": "invalid_client_secret_12345",
            "client_store_temporary_credential": False,
        }
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_AUTHORIZATION_URL", "oauth_authorization_url")
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL", "oauth_token_request_url")
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_REDIRECT_URI", "oauth_redirect_uri")
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "oauth_scope")

        # When Trying to Connect
        with pytest.raises(Exception) as exc_info:
            connection_factory(**kwargs)

        # Then Connection fails with an authentication / login error
        from snowflake.connector.errors import DatabaseError

        assert isinstance(exc_info.value, DatabaseError), f"Expected DatabaseError, got: {type(exc_info.value)}"


# ---------------------------------------------------------------------------
# OAuth Client Credentials (CC) flow
# ---------------------------------------------------------------------------


class TestOAuthClientCredentials:
    def test_oauth_should_authenticate_using_client_credentials_flow(self, connection_factory, oauth_params):
        client_id = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID")
        client_secret = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_CLIENT_SECRET")
        token_url = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL")

        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and
        #       an external IdP token URL. Snowflake's GS does not mint CC tokens,
        #       so `oauth_token_request_url` is required up-front.
        kwargs: dict[str, object] = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "oauth_client_id": client_id,
            "oauth_client_secret": client_secret,
            "oauth_token_request_url": token_url,
        }
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "oauth_scope")

        # When Trying to Connect
        connection = connection_factory(**kwargs)

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_oauth_should_fail_client_credentials_flow_with_bad_client_secret(self, connection_factory, oauth_params):
        client_id = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_CLIENT_ID")
        token_url = _require_oauth_param(oauth_params, "SNOWFLAKE_TEST_OAUTH_TOKEN_REQUEST_URL")

        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid
        #       client secret and a valid token_request_url
        kwargs: dict[str, object] = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "oauth_client_id": client_id,
            "oauth_client_secret": "invalid_client_secret_12345",
            "oauth_token_request_url": token_url,
        }
        _add_oauth_optional(kwargs, oauth_params, "SNOWFLAKE_TEST_OAUTH_SCOPE", "oauth_scope")

        # When Trying to Connect
        with pytest.raises(Exception) as exc_info:
            connection_factory(**kwargs)

        # Then Connection fails with an authentication / login error
        from snowflake.connector.errors import DatabaseError

        assert isinstance(exc_info.value, DatabaseError), f"Expected DatabaseError, got: {type(exc_info.value)}"
