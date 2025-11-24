import pytest

from .auth_helpers import verify_simple_query_execution, verify_login_error
from ...compatibility import OLD_DRIVER_ONLY, NEW_DRIVER_ONLY
from ...private_key_helper import (
    get_private_key_from_parameters,
    get_private_key_password,
    get_test_private_key_path,
)


class TestPrivateKeyAuthentication:

    def test_should_authenticate_using_private_file_with_password(
        self, connection_factory
    ):
        # Given Authentication is set to JWT and private file with password is provided
        private_key_path = get_private_key_from_parameters()
        private_key_password = get_private_key_password()

        # When Trying to Connect
        connection = create_jwt_connection(
            connection_factory,
            private_key_path,
            private_key_password
        )

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)


    def test_should_fail_jwt_authentication_when_invalid_private_key_provided(
        self, connection_factory
    ):
        # Given Authentication is set to JWT and invalid private key file is provided
        invalid_private_key_file = get_test_private_key_path()
        
        # When Trying to Connect
        with pytest.raises(Exception) as exception:
            create_jwt_connection(
                connection_factory,
                invalid_private_key_file,
            )

        # Then There is error returned
        verify_login_error(exception)


def create_jwt_connection(connection_factory, private_key_file, private_key_password=None):
    if OLD_DRIVER_ONLY("BD#5"):
        kwargs = {
            "authenticator": "SNOWFLAKE_JWT",
            "private_key_file": private_key_file,
        }
        if private_key_password:
            kwargs["private_key_file_pwd"] = private_key_password
    elif NEW_DRIVER_ONLY("BD#5"):
        kwargs = {
            "authenticator": "SNOWFLAKE_JWT", 
            "private_key_file": private_key_file,
        }
        if private_key_password:
            kwargs["private_key_password"] = private_key_password
    
    return connection_factory(**kwargs)
