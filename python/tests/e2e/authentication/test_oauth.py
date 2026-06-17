"""End-to-end OAuth tests covering the legacy access-token, Authorization Code, and Client Credentials flows.

Per-flow descriptions live in the section header blocks below.
"""

from __future__ import annotations

import pytest

from snowflake.connector.errors import DatabaseError

from ...config import get_test_parameters
from .auth_helpers import (
    connect_with_browser_automation,
    retrieve_oauth_access_token,
    verify_login_error,
    verify_simple_query_execution,
)


# ---------------------------------------------------------------------------
# Legacy AUTHENTICATOR=OAUTH (pre-acquired access token)
#
# A pre-acquired OAuth access token is passed via `token=` and presented to
# Snowflake as-is.
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def legacy_oauth_params():
    params = get_test_parameters()

    token_url = params.get("SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL")
    client_id = params.get("SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_ID")
    client_secret = params.get("SNOWFLAKE_TEST_OKTA_OAUTH_CLIENT_SECRET")
    user = params.get("SNOWFLAKE_TEST_OKTA_USER")
    password = params.get("SNOWFLAKE_TEST_OKTA_PASSWORD")
    role = params.get("SNOWFLAKE_TEST_ROLE")

    if not all([token_url, client_id, client_secret, user, password, role]):
        pytest.fail("OAuth parameters not configured.")

    return {
        "token_url": token_url,
        "client_id": client_id,
        "client_secret": client_secret,
        "user": user,
        "password": password,
        "role": role,
    }


@pytest.mark.requires_browser
class TestLegacyOAuthAccessToken:
    def test_oauth_should_authenticate_with_pre_acquired_access_token(self, connection_factory, legacy_oauth_params):
        # Given Authentication is set to legacy OAUTH and a pre-acquired OAuth access token is supplied via `token=`
        connect_params = {
            "authenticator": "OAUTH",
            "user": legacy_oauth_params["user"],
            "token": retrieve_oauth_access_token(**legacy_oauth_params),
        }

        # When Trying to Connect
        connection = connection_factory(**connect_params)

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_oauth_should_fail_legacy_authentication_with_invalid_token(self, connection_factory, legacy_oauth_params):
        # Given Authentication is set to legacy OAUTH and an invalid OAuth access token is supplied
        connect_params = {
            "authenticator": "OAUTH",
            "user": legacy_oauth_params["user"],
            "token": "invalid_oauth_token_12345",
        }

        # When Trying to Connect
        with pytest.raises(DatabaseError) as exc_info:
            connection_factory(**connect_params)

        # Then Connection fails with an authentication / login error
        verify_login_error(exc_info, ["invalid oauth access token"])

    def test_oauth_should_authenticate_using_lowercase_oauth_authenticator(
        self, connection_factory, legacy_oauth_params
    ):
        # Given Authentication is set to lowercase oauth and a valid pre-acquired
        #       OAuth access token is supplied via TOKEN
        connect_params = {
            "authenticator": "oauth",
            "user": legacy_oauth_params["user"],
            "token": retrieve_oauth_access_token(**legacy_oauth_params),
        }

        # When Trying to Connect
        connection = connection_factory(**connect_params)

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)


# ---------------------------------------------------------------------------
# OAuth Authorization Code (AC) flow
#
# An interactive, user-based flow that authenticates a real user through a
# browser login leg.
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def authorization_code_params():
    params = get_test_parameters()

    client_id = params.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_ID")
    client_secret = params.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_CLIENT_SECRET")
    redirect_uri = params.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_REDIRECT_URI")
    user = params.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_USER")
    password = params.get("SNOWFLAKE_TEST_OAUTH_SNOWFLAKE_PASSWORD")

    if not all([client_id, client_secret, redirect_uri, user, password]):
        pytest.fail("OAuth parameters not configured.")

    return {
        "user": user,
        "password": password,
        "client_id": client_id,
        "client_secret": client_secret,
        "redirect_uri": redirect_uri,
    }


@pytest.mark.requires_browser
class TestOAuthAuthorizationCode:
    def test_oauth_should_authenticate_using_authorization_code_flow(
        self, connection_factory, authorization_code_params
    ):
        # Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id / secret.
        #       `oauth_authorization_url` and `oauth_token_request_url` are forwarded from parameters
        #       when present (otherwise the driver falls back to the Snowflake-IdP defaults
        #       `https://{host}/oauth/authorize` and `https://{host}/oauth/token-request`).
        #       `client_store_temporary_credential=true` lets the AC flow short-circuit on subsequent
        #       runs by re-using the cached access / refresh token (AC state machine: cache → refresh → interactive).
        connect_params = {
            "authenticator": "OAUTH_AUTHORIZATION_CODE",
            "user": authorization_code_params["user"],
            "oauth_client_id": authorization_code_params["client_id"],
            "oauth_client_secret": authorization_code_params["client_secret"],
            "oauth_redirect_uri": authorization_code_params["redirect_uri"],
        }

        # When Trying to Connect (this will spawn the local-loopback HTTP listener and
        #      `xdg-open`/`open`/`ShellExecute` the IdP login URL unless a previously cached
        #      access token short-circuits the leg)
        connection = connect_with_browser_automation(
            connect_fn=lambda: connection_factory(**connect_params),
            scenario="internalOauthSnowflakeSuccess",
            login=authorization_code_params["user"],
            password=authorization_code_params["password"],
        )

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_oauth_should_fail_authorization_code_flow_with_bad_client_secret(
        self, connection_factory, authorization_code_params
    ):
        # Given Authentication is set to OAUTH_AUTHORIZATION_CODE with a valid client id but a
        #       deliberately invalid client secret. The IdP token-exchange step must reject the
        #       credentials and the driver must surface an authentication / login error.
        connect_params = {
            "authenticator": "OAUTH_AUTHORIZATION_CODE",
            "user": authorization_code_params["user"],
            "oauth_client_id": authorization_code_params["client_id"],
            "oauth_client_secret": "invalid_client_secret_12345",
            "oauth_redirect_uri": authorization_code_params["redirect_uri"],
        }

        # When Trying to Connect
        with pytest.raises(DatabaseError) as exc_info:
            connect_with_browser_automation(
                connect_fn=lambda: connection_factory(**connect_params),
                scenario="internalOauthSnowflakeSuccess",
                login=authorization_code_params["user"],
                password=authorization_code_params["password"],
            )

        # Then Connection fails with an authentication / login error
        verify_login_error(exc_info, ["invalid_client"])


# ---------------------------------------------------------------------------
# OAuth Client Credentials (CC) flow
#
# A non-interactive, machine-to-machine flow where an external IdP mints the
# token from a client id / secret.
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def client_credentials_params():
    params = get_test_parameters()

    token_url = params.get("SNOWFLAKE_TEST_OKTA_OAUTH_TOKEN_URL")
    client_id = params.get("SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_ID")
    client_secret = params.get("SNOWFLAKE_TEST_OKTA_OAUTH_EXTERNAL_CLIENT_SECRET")
    scope = "session:role:public"

    if not all([token_url, client_id, client_secret]):
        pytest.fail("OAuth parameters not configured.")

    return {
        "token_url": token_url,
        "client_id": client_id,
        "client_secret": client_secret,
        "scope": scope,
    }


@pytest.mark.requires_browser
class TestOAuthClientCredentials:
    def test_oauth_should_authenticate_using_client_credentials_flow(
        self, connection_factory, client_credentials_params
    ):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id / secret and
        #       an external IdP token URL. Snowflake's GS does not mint CC tokens,
        #       so `oauth_token_request_url` is required up-front.
        connect_params = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "user": client_credentials_params["client_id"],
            "oauth_client_id": client_credentials_params["client_id"],
            "oauth_client_secret": client_credentials_params["client_secret"],
            "oauth_token_request_url": client_credentials_params["token_url"],
            "oauth_scope": client_credentials_params["scope"],
        }

        # When Trying to Connect
        connection = connection_factory(**connect_params)

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_oauth_should_fail_client_credentials_flow_with_bad_client_secret(
        self, connection_factory, client_credentials_params
    ):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a valid client id, an invalid
        #       client secret and a valid token_request_url
        connect_params = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "user": client_credentials_params["client_id"],
            "oauth_client_id": client_credentials_params["client_id"],
            "oauth_client_secret": "invalid_client_secret_12345",
            "oauth_token_request_url": client_credentials_params["token_url"],
            "oauth_scope": client_credentials_params["scope"],
        }

        # When Trying to Connect
        with pytest.raises(DatabaseError) as exc_info:
            connection_factory(**connect_params)

        # Then Connection fails with an authentication / login error
        verify_login_error(exc_info, ["invalid_client"])
