import pytest


# This test verifies that unrecognized connection options are forwarded as
# session parameters in the login request — a feature of sf_core that the
# reference driver does not support.
@pytest.mark.skip_reference(reason="Reference driver does not forward unknown options as session parameters")
class TestSessionParametersViaConnectionOptions:
    def test_should_forward_unrecognized_connection_option_as_session_parameter(self, connection_factory):
        """Unrecognized kwargs should become session parameters at login."""
        # Given Snowflake client is logged in with connection option TIMEZONE
        # set to "Europe/Warsaw"
        with connection_factory(TIMEZONE="Europe/Warsaw") as conn:
            cursor = conn.cursor()
            # When Query "SHOW PARAMETERS LIKE 'TIMEZONE'" is executed
            cursor.execute("SHOW PARAMETERS LIKE 'TIMEZONE'")
            row = cursor.fetchone()
            # Then the session parameter value should be "Europe/Warsaw"
            assert row[1] == "Europe/Warsaw"  # SHOW PARAMETERS: value is column index 1
