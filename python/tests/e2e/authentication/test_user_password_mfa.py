import pytest

from ...compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from ...config import get_test_parameters
from .auth_helpers import verify_simple_query_execution


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def mfa_params():
    """
    Read MFA-specific test parameters from the environment / parameters.json.

    Required keys:
        SNOWFLAKE_TEST_MFA_USER      – account user that has MFA enabled
        SNOWFLAKE_TEST_MFA_PASSWORD  – password for that user

    Optional keys:
        SNOWFLAKE_TEST_MFA_PASSCODE  – current TOTP code (needed for passcode-based tests)
    """
    params = get_test_parameters()
    user = params.get("SNOWFLAKE_TEST_MFA_USER")
    password = params.get("SNOWFLAKE_TEST_MFA_PASSWORD")

    if not user or not password:
        pytest.skip("MFA test credentials not configured. Set SNOWFLAKE_TEST_MFA_USER and SNOWFLAKE_TEST_MFA_PASSWORD.")

    return {
        "user": user,
        "password": password,
        "passcode": params.get("SNOWFLAKE_TEST_MFA_PASSCODE"),
    }


@pytest.fixture(scope="module")
def mfa_passcode(mfa_params):
    """Skip the test when no TOTP passcode is available."""
    passcode = mfa_params["passcode"]
    if not passcode:
        pytest.skip("No MFA passcode configured. Set SNOWFLAKE_TEST_MFA_PASSCODE.")
    return passcode


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestUserPasswordMfaAuthentication:
    # ------------------------------------------------------------------
    # Passcode flow
    # ------------------------------------------------------------------

    def test_should_authenticate_using_username_password_and_totp_passcode(
        self, connection_factory, mfa_params, mfa_passcode
    ):
        # Given Authentication is set to username_password_mfa and user, password and passcode are provided
        user = mfa_params["user"]
        password = mfa_params["password"]

        # When Trying to Connect
        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=user,
            password=password,
            passcode=mfa_passcode,
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_using_username_password_with_appended_totp_passcode(
        self, connection_factory, mfa_params, mfa_passcode
    ):
        # Given Authentication is set to username_password_mfa and user, password with appended
        # passcode are provided and passcodeInPassword is set
        combined_password = mfa_params["password"] + mfa_passcode

        # When Trying to Connect
        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=mfa_params["user"],
            password=combined_password,
            passcode_in_password=True,
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    # ------------------------------------------------------------------
    # Token caching flow
    # ------------------------------------------------------------------

    def test_should_cache_mfa_token_on_first_connection(self, connection_factory, mfa_params, mfa_passcode):
        # Given Authentication is set to username_password_mfa with client_store_temporary_credential enabled
        user = mfa_params["user"]
        password = mfa_params["password"]

        # BD#16: The old driver uses client_request_mfa_token to enable MFA token caching;
        # the new driver renamed this parameter to client_store_temporary_credential.
        # The new driver also accepts client_request_mfa_token as a backward-compatible alias.
        if OLD_DRIVER_ONLY("BD#16"):
            extra = {"client_request_mfa_token": True}
        elif NEW_DRIVER_ONLY("BD#16"):
            extra = {"client_store_temporary_credential": True}

        # When Trying to Connect
        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=user,
            password=password,
            passcode=mfa_passcode,
            **extra,
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_reuse_cached_mfa_token_without_passcode(self, connection_factory, mfa_params, mfa_passcode):
        # Given Authentication is set to username_password_mfa and MFA token has been cached from a previous connection
        user = mfa_params["user"]
        password = mfa_params["password"]

        # BD#16: The old driver uses client_request_mfa_token to enable MFA token caching;
        # the new driver renamed this parameter to client_store_temporary_credential.
        # The new driver also accepts client_request_mfa_token as a backward-compatible alias.
        if OLD_DRIVER_ONLY("BD#16"):
            extra = {"client_request_mfa_token": True}
        elif NEW_DRIVER_ONLY("BD#16"):
            extra = {"client_store_temporary_credential": True}

        first = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=user,
            password=password,
            passcode=mfa_passcode,
            **extra,
        )
        with first:
            verify_simple_query_execution(first)

        # When Trying to Connect without passcode
        second = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=user,
            password=password,
            **extra,
        )

        # Then Login is successful and simple query can be executed
        with second:
            verify_simple_query_execution(second)

    # ------------------------------------------------------------------
    # Error cases
    # ------------------------------------------------------------------

    def test_should_fail_authentication_when_wrong_password_is_provided(
        self, connection_factory, mfa_params, mfa_passcode
    ):
        # Given Authentication is set to username_password_mfa and user is provided but password is skipped or invalid
        user = mfa_params["user"]

        # When Trying to Connect
        with pytest.raises(Exception) as exc_info:
            connection_factory(
                authenticator="USERNAME_PASSWORD_MFA",
                user=user,
                password="wrong_password",
                passcode=mfa_passcode,
            )

        # Then There is error returned
        from snowflake.connector.errors import DatabaseError

        assert isinstance(exc_info.value, DatabaseError), f"Expected DatabaseError, got: {type(exc_info.value)}"

    # ------------------------------------------------------------------
    # DUO push flow
    # ------------------------------------------------------------------

    @pytest.mark.skip(reason="DUO push requires interactive device approval \u2013 run manually")
    def test_should_authenticate_using_username_password_and_duo_push(self, connection_factory, mfa_params):
        # Given Authentication is set to username_password_mfa and user, password are provided and DUO push is enabled
        user = mfa_params["user"]
        password = mfa_params["password"]

        # When Trying to Connect
        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=user,
            password=password,
            ext_authn_duo_method="push",
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)
