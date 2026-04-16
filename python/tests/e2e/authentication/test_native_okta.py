import pytest

from ...config import get_test_parameters
from .auth_helpers import verify_login_error, verify_simple_query_execution


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def okta_params():
    """
    Read Okta-specific test parameters from parameters.json.

    Required keys:
        SNOWFLAKE_TEST_OKTA_HOST     – Snowflake host for Okta-enabled account
        SNOWFLAKE_TEST_OKTA_ACCOUNT  – Snowflake account for Okta-enabled account
        SNOWFLAKE_TEST_OKTA_USER     – Okta user name
        SNOWFLAKE_TEST_OKTA_PASSWORD – Okta user password
        SNOWFLAKE_TEST_OKTA_URL      – Okta authenticator URL (https://xxx.okta.com)
    """
    params = get_test_parameters()
    host = params.get("SNOWFLAKE_TEST_OKTA_HOST")
    account = params.get("SNOWFLAKE_TEST_OKTA_ACCOUNT")
    user = params.get("SNOWFLAKE_TEST_OKTA_USER")
    password = params.get("SNOWFLAKE_TEST_OKTA_PASSWORD")
    okta_url = params.get("SNOWFLAKE_TEST_OKTA_URL")

    if not all([host, account, user, password, okta_url]):
        pytest.skip(
            "Okta test credentials not configured. "
            "Set SNOWFLAKE_TEST_OKTA_HOST, SNOWFLAKE_TEST_OKTA_ACCOUNT, "
            "SNOWFLAKE_TEST_OKTA_USER, SNOWFLAKE_TEST_OKTA_PASSWORD, "
            "and SNOWFLAKE_TEST_OKTA_URL."
        )

    return {
        "host": host,
        "account": account,
        "user": user,
        "password": password,
        "okta_url": okta_url,
    }


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


@pytest.mark.skip_reference(reason="Bug in reference connector: SNOW-3388171")
class TestNativeOktaAuthentication:
    def test_should_authenticate_using_native_okta(self, connection_factory, okta_params):
        # Given Okta authentication is configured with valid credentials
        host = okta_params["host"]
        account = okta_params["account"]
        user = okta_params["user"]
        password = okta_params["password"]
        okta_url = okta_params["okta_url"]

        # When Trying to Connect
        connection = connection_factory(
            host=host,
            account=account,
            user=user,
            password=password,
            authenticator=okta_url,
            role="PUBLIC",
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_fail_native_okta_authentication_with_wrong_credentials(self, connection_factory, okta_params):
        # Given Okta authentication is configured with wrong password
        host = okta_params["host"]
        account = okta_params["account"]
        user = okta_params["user"]
        okta_url = okta_params["okta_url"]

        # When Trying to Connect
        with pytest.raises(Exception) as exception:
            connection_factory(
                host=host,
                account=account,
                user=user,
                password="wrong_password_12345",
                authenticator=okta_url,
                role="PUBLIC",
            )

        # Then Connection fails with authentication error
        verify_login_error(exception, keywords=["okta"])

    def test_should_fail_native_okta_authentication_with_wrong_okta_url(self, connection_factory, okta_params):
        # Given Okta authentication is configured with invalid okta url
        host = okta_params["host"]
        account = okta_params["account"]
        user = okta_params["user"]
        password = okta_params["password"]

        # When Trying to Connect
        with pytest.raises(Exception) as exception:
            connection_factory(
                host=host,
                account=account,
                user=user,
                password=password,
                authenticator="https://invalid.okta.com",
                role="PUBLIC",
            )

        # Then Connection fails with authentication error
        verify_login_error(exception, keywords=["authenticator"])
