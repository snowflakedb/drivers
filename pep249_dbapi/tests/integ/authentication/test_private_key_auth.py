class TestPrivateKeyAuthentication:

    def test_should_fail_jwt_authentication_when_no_private_file_provided(
        self, int_test_connection_factory
    ):
        # Given Authentication is set to JWT
        authenticator="SNOWFLAKE_JWT"

        # When Trying to Connect with no private file provided
        exception = None
        try:
            int_test_connection_factory(authenticator=authenticator)
        except Exception as e:
            exception = e

        # Then There is error returned
        self._verify_missing_parameter_error(exception)
    
    def _verify_missing_parameter_error(self, exception):
        """Verify that an exception contains a valid missing parameter error."""
        assert exception is not None
        assert str(exception).strip() != "", "Missing parameter error message should not be empty"
        assert hasattr(exception, 'error') and exception.error.missingParameter is not None, "Expected missing parameter error"
        assert exception.error.missingParameter.parameter.strip() != "", "Missing parameter name should not be empty"
