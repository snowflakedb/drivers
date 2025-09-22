import pytest
from ...compatibility import OLD_DRIVER_ONLY, NEW_DRIVER_ONLY


class TestPrivateKeyAuthentication:

    def test_should_fail_jwt_authentication_when_no_private_file_provided(
        self, int_test_connection_factory
    ):
        # Given Authentication is set to JWT
        authenticator="SNOWFLAKE_JWT"

        # When Trying to Connect with no private file provided
        with pytest.raises(Exception) as exception:
            int_test_connection_factory(authenticator=authenticator)

        # Then There is error returned
        self._verify_missing_parameter_error(exception)
    
    def _verify_missing_parameter_error(self, exception):
        assert exception is not None
        assert str(exception.value).strip() != "", "Missing parameter error message should not be empty"
        if NEW_DRIVER_ONLY("BC#4"):
            assert hasattr(exception.value, 'error') and exception.value.error.missingParameter is not None, "Expected missing parameter error"
            assert exception.value.error.missingParameter.parameter.strip() != "", "Missing parameter name should not be empty"
        if OLD_DRIVER_ONLY("BC#4"):
            assert isinstance(exception.value, TypeError), "Old driver throws TypeError for missing private key"
            error_msg = str(exception.value).lower()
            assert any(keyword in error_msg for keyword in ['private', 'key', 'missing']), \
                f"Expected error related to missing private key parameters, got: {exception.value}"

