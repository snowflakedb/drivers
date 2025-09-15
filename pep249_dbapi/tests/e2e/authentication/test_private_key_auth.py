import tempfile
import os
from contextlib import contextmanager
from typing import Optional

from .auth_helpers import verify_simple_query_execution, verify_login_error, verify_missing_parameter_error
from ...connector_factory import get_test_parameters


class TestPrivateKeyAuthentication:

    def test_should_authenticate_using_private_file_with_password(
        self, connection_factory
    ):
        # Given Authentication is set to JWT and private file with password is provided
        private_key_context = private_key_file_for_test(valid=True)
        private_key_password = get_private_key_password()

        # When Trying to Connect
        with private_key_context as private_key_file:
            connection = connection_factory(
                authenticator="SNOWFLAKE_JWT",
                private_key_file=private_key_file,
                private_key_password=private_key_password,
            )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_fail_jwt_authentication_when_no_private_file_provided(
        self, connection_factory
    ):
        # Given Authentication is set to JWT
        authenticator="SNOWFLAKE_JWT"

        # When Trying to Connect with no private file provided
        exception = None
        try:
            connection_factory(authenticator=authenticator)
        except Exception as e:
            exception = e

        # Then There is error returned
        verify_missing_parameter_error(exception)

    def test_should_fail_jwt_authentication_when_invalid_private_key_provided(
        self, connection_factory
    ):
        # Given Authentication is set to JWT and invalid private key file is provided
        invalid_key_context = private_key_file_for_test(valid=False)

        # When Trying to Connect
        exception = None
        try:
            with invalid_key_context as invalid_private_key_file:
                connection_factory(
                    authenticator="SNOWFLAKE_JWT",
                    private_key_file=invalid_private_key_file,
                )
        except Exception as e:
            exception = e

        # Then There is error returned
        verify_login_error(exception)


class PrivateKeyHelper:

    def __init__(self):
        self.key_file: Optional[tempfile.NamedTemporaryFile] = None

    def create_valid_key_file(self) -> str:
        test_params = get_test_parameters()
        private_key_contents = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")

        if not private_key_contents:
            raise RuntimeError(
                "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found in test parameters"
            )

        self.key_file = tempfile.NamedTemporaryFile(
            mode="w", suffix=".p8", delete=False
        )

        key_content = "\n".join(private_key_contents) + "\n"
        self.key_file.write(key_content)
        self.key_file.flush()

        return self.key_file.name

    def create_invalid_key_file(self) -> str:
        test_params = get_test_parameters()
        invalid_key_contents = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_INVALID")

        if not invalid_key_contents:
            raise RuntimeError(
                "SNOWFLAKE_TEST_PRIVATE_KEY_INVALID not found in test parameters"
            )

        self.key_file = tempfile.NamedTemporaryFile(
            mode="w", suffix=".p8", delete=False
        )

        key_content = "\n".join(invalid_key_contents) + "\n"
        self.key_file.write(key_content)
        self.key_file.flush()

        return self.key_file.name

    def cleanup(self):
        if self.key_file:
            try:
                self.key_file.close()
                os.unlink(self.key_file.name)
            except Exception:
                # Ignore cleanup errors in tests
                pass
            finally:
                self.key_file = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.cleanup()


def get_private_key_password() -> str:
    test_params = get_test_parameters()
    password = test_params.get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")

    if not password:
        raise RuntimeError(
            "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD not found in test parameters"
        )

    return password


@contextmanager
def private_key_file_for_test(valid: bool = True):
    with PrivateKeyHelper() as key_manager:
        if valid:
            key_file_path = key_manager.create_valid_key_file()
        else:
            key_file_path = key_manager.create_invalid_key_file()
        yield key_file_path
