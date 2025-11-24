import pytest
import random
import logging

from .auth_helpers import verify_simple_query_execution, verify_login_error
from ...connector_factory import get_test_parameters


@pytest.fixture
def pat_token(connection_factory):
    pat_token = PAT(connection_factory)
    try:
        token = pat_token.acquire_token()
        yield token
    finally:
        pat_token.cleanup()


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

    def test_should_fail_pat_authentication_when_invalid_token_provided(
        self, connection_factory
    ):
        # Given Authentication is set to Programmatic Access Token and invalid PAT token is provided
        authenticator = "PROGRAMMATIC_ACCESS_TOKEN"
        invalid_token = get_invalid_pat_token()

        # When Trying to Connect
        with pytest.raises(Exception) as exception:
            connection_factory(authenticator=authenticator, token=invalid_token)

        # Then There is error returned
        verify_login_error(exception)


class PAT:
    def __init__(self, connection_factory):
        self.connection_factory = connection_factory
        self._token_name = None
        self._token_secret = None

    def acquire_token(self) -> str:
        token_name = f"pat_{random.randint(0, 2**32-1):x}"
        test_params = get_test_parameters()
        user = test_params.get("SNOWFLAKE_TEST_USER")
        role = test_params.get("SNOWFLAKE_TEST_ROLE")

        with self.connection_factory() as connection:
            with connection.cursor() as cursor:
                sql = f"ALTER USER IF EXISTS {user} ADD PROGRAMMATIC ACCESS TOKEN {token_name} ROLE_RESTRICTION = {role}"
                cursor.execute(sql)
                result = cursor.fetchone()

                if result and len(result) >= 2:
                    self._token_name = token_name
                    self._token_secret = result[1]
                    return self._token_secret
                else:
                    raise RuntimeError(
                        "Failed to create PAT token - unexpected result format"
                    )

    def cleanup(self):
        if self._token_name:
            test_params = get_test_parameters()
            user = test_params.get("SNOWFLAKE_TEST_USER")

            try:
                with self.connection_factory() as connection:
                    with connection.cursor() as cursor:
                        sql = f"ALTER USER IF EXISTS {user} REMOVE PROGRAMMATIC ACCESS TOKEN {self._token_name}"
                        cursor.execute(sql)
            except Exception as e:
                logging.warning(f"Failed to cleanup PAT token {self._token_name}: {e}")
                pass
            finally:
                self._token_name = None
                self._token_secret = None

def get_invalid_pat_token() -> str:
    return "invalid_token_12345"
