import pytest

from snowflake.connector import ProgrammingError

from ...compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY


class TestPrivateKeyAuthentication:
    def test_should_fail_jwt_authentication_when_no_private_file_provided(self, int_test_connection_factory):
        # Given Authentication is set to JWT
        authenticator = "SNOWFLAKE_JWT"

        # When Trying to Connect with no private file provided
        with pytest.raises(Exception) as exception_info:
            int_test_connection_factory(authenticator=authenticator, private_key_file=None)

        exception = exception_info.value

        # Then There is error returned
        assert exception is not None

        if NEW_DRIVER_ONLY("BD#4"):
            assert isinstance(exception, ProgrammingError)
            assert "Missing required parameter: private_key or private_key_file" in str(exception)
        elif OLD_DRIVER_ONLY("BD#4"):
            assert isinstance(exception, TypeError)
            assert "Expected bytes or RSAPrivateKey" in str(exception)
