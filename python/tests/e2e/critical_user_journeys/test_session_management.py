"""Session Management E2E tests for Universal Driver.

This module tests session management functionality including:
- Setting and verifying session parameters (QUERY_TAG, TIMEZONE)
- Altering session parameters at runtime
- Switching role and restoring original
- Switching and restoring schema context
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestSessionManagement:
    """Tests for session parameter setting and context switching."""

    def test_should_set_and_verify_session_parameter_query_tag(self, execute_query, cursor):
        """Test setting and verifying session parameter QUERY_TAG."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Session parameter QUERY_TAG is set to "e2e_test" via ALTER SESSION
        cursor.execute("ALTER SESSION SET QUERY_TAG = 'e2e_test'")

        # Then SHOW PARAMETERS LIKE 'QUERY_TAG' should return value "e2e_test"
        cursor.execute("SHOW PARAMETERS LIKE 'QUERY_TAG'")
        result = cursor.fetchone()
        assert result is not None
        # Result format: key, value, default, level, description, type
        assert result[1] == "e2e_test"

    def test_should_set_and_verify_session_parameter_timezone(self, execute_query, cursor):
        """Test setting and verifying session parameter TIMEZONE."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Session parameter TIMEZONE is set to "America/New_York" via ALTER SESSION
        cursor.execute("ALTER SESSION SET TIMEZONE = 'America/New_York'")

        # Then SHOW PARAMETERS LIKE 'TIMEZONE' should return value "America/New_York"
        cursor.execute("SHOW PARAMETERS LIKE 'TIMEZONE'")
        result = cursor.fetchone()
        assert result is not None
        assert result[1] == "America/New_York"

    def test_should_alter_session_parameter_at_runtime(self, execute_query, cursor):
        """Test altering session parameter at runtime."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Session parameter TIMEZONE is set to "America/New_York"
        cursor.execute("ALTER SESSION SET TIMEZONE = 'America/New_York'")

        # When TIMEZONE is changed to "UTC" via ALTER SESSION
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")

        # Then SHOW PARAMETERS LIKE 'TIMEZONE' should return value "UTC"
        cursor.execute("SHOW PARAMETERS LIKE 'TIMEZONE'")
        result = cursor.fetchone()
        assert result is not None
        assert result[1] == "UTC"

    def test_should_switch_role_and_restore_original(self, execute_query, cursor):
        """Test switching role and restoring original."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And The current role is recorded
        cursor.execute("SELECT CURRENT_ROLE()")
        original_role = cursor.fetchone()[0]
        assert original_role is not None

        # When USE ROLE PUBLIC is executed
        cursor.execute("USE ROLE PUBLIC")

        # Then SELECT CURRENT_ROLE() should return "PUBLIC"
        cursor.execute("SELECT CURRENT_ROLE()")
        result = cursor.fetchone()
        assert result[0] == "PUBLIC"

        # When The original role is restored
        cursor.execute(f"USE ROLE {original_role}")

        # Then SELECT CURRENT_ROLE() should return the original role
        cursor.execute("SELECT CURRENT_ROLE()")
        result = cursor.fetchone()
        assert result[0] == original_role

    def test_should_switch_and_restore_schema_context(self, execute_query, cursor):
        """Test switching and restoring schema context."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And The current schema is recorded
        cursor.execute("SELECT CURRENT_SCHEMA()")
        original_schema = cursor.fetchone()[0]
        assert original_schema is not None

        # When USE SCHEMA is executed to switch to INFORMATION_SCHEMA
        cursor.execute("USE SCHEMA INFORMATION_SCHEMA")

        # Then SELECT CURRENT_SCHEMA() should return "INFORMATION_SCHEMA"
        cursor.execute("SELECT CURRENT_SCHEMA()")
        result = cursor.fetchone()
        assert result[0] == "INFORMATION_SCHEMA"

        # When The original schema is restored via USE SCHEMA
        cursor.execute(f"USE SCHEMA {original_schema}")

        # Then SELECT CURRENT_SCHEMA() should return the original schema
        cursor.execute("SELECT CURRENT_SCHEMA()")
        result = cursor.fetchone()
        assert result[0] == original_schema
