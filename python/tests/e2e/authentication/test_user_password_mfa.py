import pytest

from ...compatibility import NEW_DRIVER_ONLY
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

    def test_should_authenticate_with_explicit_passcode(self, connection_factory, mfa_params, mfa_passcode):
        # Given Authentication is set to USERNAME_PASSWORD_MFA and a TOTP passcode is provided
        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=mfa_params["user"],
            password=mfa_params["password"],
            passcode=mfa_passcode,
        )

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_with_passcode_in_password(self, connection_factory, mfa_params, mfa_passcode):
        # Given Authentication is set to USERNAME_PASSWORD_MFA and the passcode is appended to the password
        combined_password = mfa_params["password"] + mfa_passcode

        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=mfa_params["user"],
            password=combined_password,
            passcode_in_password=True,
        )

        # Then Login is successful and a simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    # ------------------------------------------------------------------
    # Token caching flow
    # ------------------------------------------------------------------

    @pytest.mark.skipif(not NEW_DRIVER_ONLY("BD#MFA1"), reason="Token caching only supported in new driver")
    def test_should_cache_mfa_token_on_first_connection(self, connection_factory, mfa_params, mfa_passcode):
        # Given caching flags are enabled and a valid passcode is provided
        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=mfa_params["user"],
            password=mfa_params["password"],
            passcode=mfa_passcode,
            client_store_temporary_credential=True,
        )

        # Then Login is successful – the server issues an MFA token that the driver caches
        with connection:
            verify_simple_query_execution(connection)

    @pytest.mark.skipif(not NEW_DRIVER_ONLY("BD#MFA2"), reason="Token caching only supported in new driver")
    def test_should_reuse_cached_mfa_token_without_passcode(self, connection_factory, mfa_params, mfa_passcode):
        # Given the first connection has already cached an MFA token (see test above)
        # Establish first connection to warm the cache
        first = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=mfa_params["user"],
            password=mfa_params["password"],
            passcode=mfa_passcode,
            client_store_temporary_credential=True,
        )
        with first:
            verify_simple_query_execution(first)

        # When a second connection is made without a passcode
        second = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=mfa_params["user"],
            password=mfa_params["password"],
            client_store_temporary_credential=True,
        )

        # Then the cached token is used and login succeeds without prompting for MFA
        with second:
            verify_simple_query_execution(second)

    # ------------------------------------------------------------------
    # DUO push flow
    # ------------------------------------------------------------------

    @pytest.mark.skip(reason="DUO push requires interactive device approval – run manually")
    def test_should_authenticate_with_duo_push(self, connection_factory, mfa_params):
        # Given DUO push is configured as the MFA method
        connection = connection_factory(
            authenticator="USERNAME_PASSWORD_MFA",
            user=mfa_params["user"],
            password=mfa_params["password"],
            ext_authn_duo_method="push",
        )

        # Then (after approving the push on the registered device) login succeeds
        with connection:
            verify_simple_query_execution(connection)

    # ------------------------------------------------------------------
    # Error cases
    # ------------------------------------------------------------------

    def test_should_fail_with_invalid_passcode(self, connection_factory, mfa_params):
        # Given an incorrect TOTP passcode is provided
        with pytest.raises(Exception) as exc_info:
            connection_factory(
                authenticator="USERNAME_PASSWORD_MFA",
                user=mfa_params["user"],
                password=mfa_params["password"],
                passcode="000000",
            )

        # Then a DatabaseError is raised
        from snowflake.connector.errors import DatabaseError

        assert isinstance(exc_info.value, DatabaseError), f"Expected DatabaseError, got: {type(exc_info.value)}"

    def test_should_fail_with_invalid_password(self, connection_factory, mfa_params, mfa_passcode):
        # Given the password is wrong
        with pytest.raises(Exception) as exc_info:
            connection_factory(
                authenticator="USERNAME_PASSWORD_MFA",
                user=mfa_params["user"],
                password="wrong_password",
                passcode=mfa_passcode,
            )

        # Then a DatabaseError is raised
        from snowflake.connector.errors import DatabaseError

        assert isinstance(exc_info.value, DatabaseError), f"Expected DatabaseError, got: {type(exc_info.value)}"
