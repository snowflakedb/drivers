import pytest

from ...config import get_test_parameters
from .auth_helpers import verify_login_error, verify_simple_query_execution


@pytest.fixture(scope="session")
def pat_token():
    params = get_test_parameters()
    token = params.get("SNOWFLAKE_TEST_PAT")
    assert token, "SNOWFLAKE_TEST_PAT must be set in parameters.json"
    return token


class TestPATAuthentication:
    def test_should_authenticate_using_pat_as_password(self, connection_factory, pat_token):
        # Given Authentication is set to password and valid PAT token is provided
        password = pat_token

        # When Trying to Connect
        connection = connection_factory(password=password)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_using_pat_as_token(self, connection_factory, pat_token):
        # Given Authentication is set to Programmatic Access Token and valid PAT token is provided
        authenticator = "PROGRAMMATIC_ACCESS_TOKEN"
        token = pat_token

        # When Trying to Connect
        connection = connection_factory(authenticator=authenticator, token=token)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_using_pat_token_from_token_file_path(self, connection_factory, pat_token, tmp_path):
        # Given Authentication is set to Programmatic Access Token and a valid PAT token is stored in a file
        token_file = tmp_path / "pat.token"
        token_file.write_text(pat_token)
        token_file.chmod(0o600)

        # When Trying to Connect
        connection = connection_factory(
            authenticator="PROGRAMMATIC_ACCESS_TOKEN",
            token_file_path=str(token_file),
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_using_pat_as_token_without_user(self, connection_factory, pat_token):
        # Given Authentication is set to Programmatic Access Token and valid PAT token is provided
        authenticator = "PROGRAMMATIC_ACCESS_TOKEN"
        token = pat_token

        # When Trying to Connect without user
        connection = connection_factory(authenticator=authenticator, token=token, user=None)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_fail_pat_authentication_when_invalid_token_provided(self, connection_factory):
        # Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
        authenticator = "PROGRAMMATIC_ACCESS_TOKEN"
        invalid_token = get_invalid_pat_token()

        # When Trying to Connect
        with pytest.raises(Exception) as exception:
            connection_factory(authenticator=authenticator, token=invalid_token)

        # Then There is error returned
        verify_login_error(exception, keywords=["token", "invalid"])


def get_invalid_pat_token() -> str:
    return "invalid_token_12345"
