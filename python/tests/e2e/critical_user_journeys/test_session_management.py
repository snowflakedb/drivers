"""Session Management E2E tests for Universal Driver.

This module tests session management functionality including:
- Switching role and restoring original
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestSessionManagement:
    """Tests for session parameter setting and context switching."""

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
