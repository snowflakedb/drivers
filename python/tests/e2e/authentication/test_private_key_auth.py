import base64

import pytest

from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization

from ...compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY
from ...private_key_helper import (
    get_private_key_from_parameters,
    get_private_key_password,
    get_test_private_key_path,
)
from .auth_helpers import verify_login_error, verify_simple_query_execution


class TestPrivateKeyAuthentication:
    def test_should_authenticate_using_private_file_with_password(self, connection_factory):
        # Given Authentication is set to JWT and private file with password is provided
        private_key_path = get_private_key_from_parameters()
        private_key_password = get_private_key_password()

        # When Trying to Connect
        connection = create_jwt_connection(connection_factory, private_key_path, private_key_password)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_fail_jwt_authentication_when_invalid_private_key_provided(self, connection_factory):
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

    def test_should_authenticate_using_private_key_as_bytes(self, connection_factory):
        # Given Authentication is set to JWT and private key is provided as bytes
        private_key_path = get_private_key_from_parameters()
        private_key_password = get_private_key_password()

        # Load the private key from file and convert to bytes
        with open(private_key_path, "rb") as key_file:
            private_key_bytes = serialization.load_pem_private_key(
                key_file.read(),
                password=private_key_password.encode() if private_key_password else None,
                backend=default_backend(),
            )
            private_key_der = private_key_bytes.private_bytes(
                encoding=serialization.Encoding.DER,
                format=serialization.PrivateFormat.PKCS8,
                encryption_algorithm=serialization.NoEncryption(),
            )

        # When Trying to Connect
        connection = create_jwt_connection_with_private_key(connection_factory, private_key_der)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_using_private_key_as_str(self, connection_factory):
        # Given Authentication is set to JWT and private key is provided as str
        private_key_path = get_private_key_from_parameters()
        private_key_password = get_private_key_password()

        # Load the private key from file and convert to base64-encoded string
        with open(private_key_path, "rb") as key_file:
            private_key_bytes = serialization.load_pem_private_key(
                key_file.read(),
                password=private_key_password.encode() if private_key_password else None,
                backend=default_backend(),
            )
            private_key_der = private_key_bytes.private_bytes(
                encoding=serialization.Encoding.DER,
                format=serialization.PrivateFormat.PKCS8,
                encryption_algorithm=serialization.NoEncryption(),
            )
            private_key_str = base64.b64encode(private_key_der).decode()

        # When Trying to Connect
        connection = create_jwt_connection_with_private_key(connection_factory, private_key_str)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)

    def test_should_authenticate_using_private_key_as_rsaprivatekey_object(self, connection_factory):
        # Given Authentication is set to JWT and private key is provided as RSAPrivateKey object
        private_key_path = get_private_key_from_parameters()
        private_key_password = get_private_key_password()

        # Load the private key from file as RSAPrivateKey object
        with open(private_key_path, "rb") as key_file:
            private_key_obj = serialization.load_pem_private_key(
                key_file.read(),
                password=private_key_password.encode() if private_key_password else None,
                backend=default_backend(),
            )

        # When Trying to Connect
        connection = create_jwt_connection_with_private_key(connection_factory, private_key_obj)

        # Then Login is successful and simple query can be executed
        with connection:
            verify_simple_query_execution(connection)


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


def create_jwt_connection_with_private_key(connection_factory, private_key):
    """Create a JWT connection using private_key parameter instead of private_key_file.

    Args:
        connection_factory: Factory function to create connection
        private_key: Private key in one of the supported formats:
                     - bytes (DER format)
                     - str (base64-encoded DER)
                     - RSAPrivateKey object

    Returns:
        Connection object
    """
    kwargs = {
        "authenticator": "SNOWFLAKE_JWT",
        "private_key": private_key,
    }
    return connection_factory(**kwargs)
