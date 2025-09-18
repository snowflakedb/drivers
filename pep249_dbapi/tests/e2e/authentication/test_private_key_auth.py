import os
import tempfile
from contextlib import contextmanager

from .auth_helpers import verify_simple_query_execution, verify_login_error
from ...connector_factory import get_test_parameters
from ...utils import repo_root


class TestPrivateKeyAuthentication:

    def test_should_authenticate_using_private_file_with_password(
        self, connection_factory
    ):
        # Given Authentication is set to JWT and private file with password is provided
        private_key_password = get_private_key_password()

        # When Trying to Connect
        with create_valid_key_file() as private_key_file:
            connection = connection_factory(
                authenticator="SNOWFLAKE_JWT",
                private_key_file=private_key_file,
                private_key_password=private_key_password,
            )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)


    def test_should_fail_jwt_authentication_when_invalid_private_key_provided(
        self, connection_factory
    ):
        # Given Authentication is set to JWT and invalid private key file is provided
        invalid_private_key_file = get_invalid_key_file_path()
        
        # When Trying to Connect
        exception = None
        try:
            connection_factory(
                authenticator="SNOWFLAKE_JWT",
                private_key_file=invalid_private_key_file,
            )
        except Exception as e:
            exception = e

        # Then There is error returned
        verify_login_error(exception)


@contextmanager
def create_valid_key_file():
    """Create a temporary valid private key file and clean it up automatically."""
    test_params = get_test_parameters()
    private_key_contents = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")

    if not private_key_contents:
        raise RuntimeError(
            "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found in test parameters"
        )

    with tempfile.NamedTemporaryFile(mode="w", suffix=".p8", delete=False) as key_file:
        key_content = "\n".join(private_key_contents) + "\n"
        key_file.write(key_content)
        key_file.flush()
        temp_path = key_file.name
    
    try:
        yield temp_path
    finally:
        try:
            os.unlink(temp_path)
        except Exception:
            # Ignore cleanup errors in tests
            pass


def get_invalid_key_file_path() -> str:
    """Return the path to the shared invalid private key file."""
    return str(repo_root() / "tests" / "test_data" / "invalid_rsa_key.p8")


def get_private_key_password() -> str:
    test_params = get_test_parameters()
    password = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")

    if not password:
        raise RuntimeError(
            "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD not found in test parameters"
        )

    return password


