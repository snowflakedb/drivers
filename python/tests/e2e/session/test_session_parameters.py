import pytest


# This test verifies that unrecognized connection options are forwarded as
# session parameters in the login request — a feature of sf_core that the
# reference driver does not support.
@pytest.mark.skip_reference(reason="Reference driver does not forward unknown options as session parameters")
class TestSessionParametersViaConnectionOptions:
    def test_should_forward_unrecognized_connection_option_as_session_parameter(self, connection_factory):
        """Unrecognized kwargs should become session parameters at login."""
        # Given Snowflake client is logged in with connection option QUERY_TAG
        # set to "session_param_e2e_test"
        with connection_factory(QUERY_TAG="session_param_e2e_test") as conn:
            cursor = conn.cursor()
            # When Query "SELECT CURRENT_QUERY_TAG()" is executed
            cursor.execute("SELECT CURRENT_QUERY_TAG()")
            row = cursor.fetchone()
            # Then the result should contain value "session_param_e2e_test"
            assert row[0] == "session_param_e2e_test"
