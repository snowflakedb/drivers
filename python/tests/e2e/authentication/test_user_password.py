import pytest

from snowflake.connector.errors import DatabaseError

from ...config import get_test_parameters
from .auth_helpers import verify_simple_query_execution


# Some test accounts (notably GCP) enforce MFA at the account level, which
# causes plain username+password login to fail with:
#   "Multi-factor authentication is required for this account."
# This is a server-side policy, not a driver bug.  We detect this at runtime
# and skip the happy-path tests so they don't produce false failures.
MFA_ENFORCED_MESSAGE = "multi-factor authentication is required"


def get_password_auth_params():
    """Build connection params for plain password auth, overriding the default JWT."""
    test_params = get_test_parameters()
    return {
        "authenticator": "snowflake",
        "password": test_params.get("SNOWFLAKE_TEST_PASSWORD"),
    }


class TestUserPasswordAuthentication:
    def test_should_authenticate_using_username_and_password(self, connection_factory):
        # Given Authentication is set to default (snowflake) with valid username and password
        params = get_password_auth_params()

        # When Trying to Connect
        try:
            connection = connection_factory(**params)
        except Exception as e:
            if MFA_ENFORCED_MESSAGE in str(e).lower():
                pytest.skip("Account has MFA enforcement enabled — plain password auth is not possible")
            raise

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_fail_authentication_when_wrong_password_is_provided(self, connection_factory):
        # Given Authentication is set to default with valid username and wrong password
        params = get_password_auth_params()
        params["password"] = "definitely_not_a_valid_password_12345"

        # When Trying to Connect
        with pytest.raises(Exception) as exception:
            connection_factory(**params)

        # Then There is error returned
        assert isinstance(exception.value, DatabaseError), (
            f"Expected DatabaseError, got: {type(exception.value).__name__}: {exception.value}"
        )
