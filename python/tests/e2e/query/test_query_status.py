"""Query status tests for Universal Driver (Python).

This module tests query status retrieval functionality including:
- Retrieving success status for completed queries
- Retrieving error status for failed queries
- Checking still-running status for in-progress queries
- Error handling for invalid query IDs
"""

from __future__ import annotations

import pytest

from snowflake.connector import ProgrammingError


class TestQueryStatus:
    """Tests for query status retrieval."""

    @pytest.mark.skip(reason="Implementation pending")
    def test_should_return_success_status_for_completed_query(self, connection, cursor):
        """Test that a completed query returns success status."""
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 1" is executed
        cursor.execute("SELECT 1")

        # And Query status is retrieved by query ID
        status = connection.get_query_status(cursor.sfqid)

        # Then the query status should indicate success
        assert status is not None

        # And the query should not be indicated as still running
        assert not connection.is_still_running(status)

        # And the query should not be indicated as an error
        assert not connection.is_an_error(status)

    @pytest.mark.skip(reason="Implementation pending")
    def test_should_return_error_status_for_failed_query(self, connection, cursor):
        """Test that a failed query returns error status."""
        # Given Snowflake client is logged in
        pass

        # When An invalid query is executed and the query ID is captured
        with pytest.raises(ProgrammingError):
            cursor.execute("SELECT * FROM NON_EXISTENT_TABLE_TEST_12345")
        query_id = cursor.sfqid

        # And Query status is retrieved by query ID
        status = connection.get_query_status(query_id)

        # Then the query status should indicate an error
        assert connection.is_an_error(status)

        # And the query should not be indicated as still running
        assert not connection.is_still_running(status)

    @pytest.mark.skip(reason="Implementation pending")
    def test_should_indicate_still_running_for_in_progress_query(self, connection, cursor):
        """Test that an in-progress query indicates still running."""
        # Given Snowflake client is logged in
        pass

        # When A long-running query is submitted asynchronously
        cursor.execute_async("SELECT SYSTEM$WAIT(30)")
        query_id = cursor.sfqid

        # And Query status is retrieved immediately
        status = connection.get_query_status(query_id)

        # Then the query status should indicate still running
        assert connection.is_still_running(status)

        # And the query should not be indicated as an error
        assert not connection.is_an_error(status)

    @pytest.mark.skip(reason="Implementation pending")
    def test_should_raise_error_when_retrieving_status_with_invalid_query_id(self, connection):
        """Test that retrieving status with invalid query ID raises error."""
        # Given Snowflake client is logged in
        pass

        # When Query status is retrieved for a non-existent query ID
        invalid_query_id = "00000000-0000-0000-0000-000000000000"

        # Then An error should be returned
        with pytest.raises(ProgrammingError):
            connection.get_query_status(invalid_query_id)
