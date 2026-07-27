"""
Username/password MFA authentication E2E tests.

Requires the snowdrivers-test-external-browser-universal-driver Docker container
(/externalbrowser/totpGenerator.js generates TOTP passcodes for the MFA test user).

Run locally:
  ./tests/auth/run_auth_browser.sh python
"""

import os

from dataclasses import dataclass

import pytest

from snowflake.connector.errors import DatabaseError

from ...compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from .auth_helpers import acquire_totp_passcode, connect_with_totp_retry, verify_simple_query_execution
from .conftest import require_auth_params


@dataclass(frozen=True)
class MfaCredentials:
    """Username/password MFA credentials.

    ``totp_seed`` feeds the TOTP generator; it is not a connection parameter.
    """

    user: str
    password: str
    totp_seed: str

    def connect_params(self, **overrides) -> dict:
        params = {
            "user": self.user,
            "password": self.password,
            "authenticator": "USERNAME_PASSWORD_MFA",
            "role": "PUBLIC",
        }
        params.update(overrides)
        return params


@pytest.fixture(scope="module")
def mfa_credentials() -> MfaCredentials:
    """MFA test credentials. Fails if credentials are missing in browser env."""
    values = require_auth_params(
        "SNOWFLAKE_TEST_MFA_USER",
        "SNOWFLAKE_TEST_MFA_PASSWORD",
        "SNOWFLAKE_TEST_MFA_SEED",
    )
    os.environ["SNOWFLAKE_AUTH_MFA_SEED"] = values["SNOWFLAKE_TEST_MFA_SEED"]
    return MfaCredentials(
        user=values["SNOWFLAKE_TEST_MFA_USER"],
        password=values["SNOWFLAKE_TEST_MFA_PASSWORD"],
        totp_seed=values["SNOWFLAKE_TEST_MFA_SEED"],
    )


def _mfa_token_cache_params() -> dict:
    # BD#16: The old driver uses client_request_mfa_token to enable MFA token caching;
    # the new driver renamed this parameter to client_store_temporary_credential.
    # The new driver also accepts client_request_mfa_token as a backward-compatible alias.
    if OLD_DRIVER_ONLY("BD#16"):
        return {"client_request_mfa_token": True}
    if NEW_DRIVER_ONLY("BD#16"):
        return {"client_store_temporary_credential": True}
    return {}


@pytest.mark.requires_browser
class TestUserPasswordMfaAuthentication:
    # ------------------------------------------------------------------
    # Passcode flow
    # ------------------------------------------------------------------

    def test_should_authenticate_using_username_password_and_totp_passcode(self, connection_factory, mfa_credentials):
        # Given Authentication is set to username_password_mfa and user, password and passcode are provided
        connect_params = mfa_credentials.connect_params()

        # When Trying to Connect
        connection = connect_with_totp_retry(
            connection_factory,
            mfa_credentials.totp_seed,
            **connect_params,
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_using_username_password_with_appended_totp_passcode(
        self, connection_factory, mfa_credentials
    ):
        # Given Authentication is set to username_password_mfa and user, password with appended
        # passcode are provided and passcodeInPassword is set
        connect_params = mfa_credentials.connect_params()

        # When Trying to Connect
        connection = connect_with_totp_retry(
            connection_factory,
            mfa_credentials.totp_seed,
            passcode_in_password=True,
            **connect_params,
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    # ------------------------------------------------------------------
    # Token caching flow
    # ------------------------------------------------------------------

    def test_should_reuse_cached_mfa_token_without_passcode(self, connection_factory, mfa_credentials):
        # Given Authentication is set to username_password_mfa and MFA token has been cached from a previous connection
        connect_params = mfa_credentials.connect_params(**_mfa_token_cache_params())

        first = connect_with_totp_retry(
            connection_factory,
            mfa_credentials.totp_seed,
            **connect_params,
        )
        with first:
            verify_simple_query_execution(first)

        # When Trying to Connect without passcode
        second = connection_factory(**connect_params)

        # Then Login is successful and simple query can be executed
        with second:
            verify_simple_query_execution(second)

    # ------------------------------------------------------------------
    # Error cases
    # ------------------------------------------------------------------

    @pytest.mark.skip(reason="Bad-secret tests cause pipeline flakiness by blocking the test account")
    def test_should_fail_authentication_when_wrong_password_is_provided(self, connection_factory, mfa_credentials):
        # Given Authentication is set to username_password_mfa and user is provided but password is skipped or invalid
        passcode = acquire_totp_passcode(mfa_credentials.totp_seed)
        connect_params = mfa_credentials.connect_params(password="wrong_password", passcode=passcode)

        # When Trying to Connect
        with pytest.raises(Exception) as exc_info:
            connection_factory(**connect_params)

        # Then There is error returned
        assert isinstance(exc_info.value, DatabaseError), f"Expected DatabaseError, got: {type(exc_info.value)}"

    # ------------------------------------------------------------------
    # DUO push flow
    # ------------------------------------------------------------------

    @pytest.mark.skip(reason="DUO push requires interactive device approval - run manually")
    def test_should_authenticate_using_username_password_and_duo_push(self, connection_factory, mfa_credentials):
        # Given Authentication is set to username_password_mfa and user, password are provided and DUO push is enabled
        connect_params = mfa_credentials.connect_params(ext_authn_duo_method="push")

        # When Trying to Connect
        connection = connection_factory(**connect_params)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)
