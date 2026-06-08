import pytest

from snowflake.connector.errors import DatabaseError

from ...config import get_test_parameters
from .auth_helpers import verify_simple_query_execution


MFA_ENFORCED_MESSAGE = "multi-factor authentication is required"


def get_password_auth_params():
    """Build connection params for plain password auth, overriding the default JWT."""
    test_params = get_test_parameters()
    password = test_params.get("SNOWFLAKE_TEST_PASSWORD")
    if not password:
        pytest.skip("SNOWFLAKE_TEST_PASSWORD not configured (JWT-only environment)")
    return {
        "authenticator": "snowflake",
        "password": password,
    }


class TestUserPasswordAuthentication:
    def test_should_authenticate_using_username_and_password(self, connection_factory):
        # Given Authentication is set to default (snowflake) with valid username and password
        params = get_password_auth_params()

        # When Trying to Connect
        try:
            connection = connection_factory(**params)
        except DatabaseError as e:
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
        with pytest.raises(DatabaseError):
            # Then There is error returned
            connection_factory(**params)
