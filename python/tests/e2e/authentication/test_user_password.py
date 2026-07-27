import pytest

from snowflake.connector.errors import DatabaseError

from .auth_helpers import verify_simple_query_execution
from .conftest import require_auth_params


def get_password_auth_params():
    """Build connection params for plain password auth, overriding the default JWT."""
    creds = require_auth_params("SNOWFLAKE_TEST_PASSWORD")
    return {
        "authenticator": "snowflake",
        "password": creds["SNOWFLAKE_TEST_PASSWORD"],
    }


@pytest.mark.requires_no_mfa
class TestUserPasswordAuthentication:
    def test_should_authenticate_using_username_and_password(self, connection_factory):
        # Given Authentication is set to default (snowflake) with valid username and password
        params = get_password_auth_params()

        # When Trying to Connect
        connection = connection_factory(**params)

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
