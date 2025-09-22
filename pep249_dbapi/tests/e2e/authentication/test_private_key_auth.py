import pytest
import tempfile
from contextlib import contextmanager
from pathlib import Path

from .auth_helpers import verify_simple_query_execution, verify_login_error
from ...connector_factory import get_test_parameters
from ...utils import repo_root
from ...compatibility import OLD_DRIVER_ONLY, NEW_DRIVER_ONLY


class TestPrivateKeyAuthentication:

    def test_should_authenticate_using_private_file_with_password(
        self, connection_factory
    ):
        # Given Authentication is set to JWT and private file with password is provided
        private_key_password = get_private_key_password()

        # When Trying to Connect
        with create_valid_key_file() as private_key_file:
            connection = create_jwt_connection(
                connection_factory,
                private_key_file,
                private_key_password
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
        with pytest.raises(Exception) as exception:
            create_jwt_connection(
                connection_factory,
                invalid_private_key_file,
            )

        # Then There is error returned
        verify_login_error(exception)


def create_jwt_connection(connection_factory, private_key_file, private_key_password=None):
    if OLD_DRIVER_ONLY("BC#5"):
        kwargs = {
            "authenticator": "SNOWFLAKE_JWT",
            "private_key_file": private_key_file,
        }
        if private_key_password:
            kwargs["private_key_file_pwd"] = private_key_password
    elif NEW_DRIVER_ONLY("BC#5"):
        kwargs = {
            "authenticator": "SNOWFLAKE_JWT", 
            "private_key_file": private_key_file,
        }
        if private_key_password:
            kwargs["private_key_password"] = private_key_password
    
    return connection_factory(**kwargs)

@contextmanager
def create_valid_key_file():
    """Create a temporary valid private key file and clean it up automatically."""
    test_params = get_test_parameters()
    private_key_contents = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")

    if not private_key_contents:
        raise RuntimeError(
            "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found in test parameters"
        )

    with tempfile.TemporaryDirectory() as tmp_dir:
        key_file = Path(tmp_dir) / "key.p8"
        key_file.write_text("\n".join(private_key_contents) + "\n")
        yield str(key_file)


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
