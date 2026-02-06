"""
Integration tests for async execution.

Tests execute_async(), get_results_from_sfqid(), and query status checking.
"""

import time

import pytest

from snowflake.connector.constants import QueryStatus
from snowflake.connector.errors import DatabaseError, NotSupportedError


def test_execute_async_simple_query(connection):
    """Test basic execute_async with a simple query."""
    with connection.cursor() as cur:
        # Execute async query
        result = cur.execute_async("SELECT 1 AS test_col")

        # Verify result contains query ID
        assert "queryId" in result
        assert result["queryId"] is not None
        assert cur.sfqid is not None
        assert cur.sfqid == result["queryId"]


def test_get_results_from_sfqid(connection):
    """Test retrieving results from a query ID."""
    with connection.cursor() as cur:
        # Execute async query
        result = cur.execute_async("SELECT 42 AS answer, 'hello' AS greeting")
        sfqid = result["queryId"]

        # Wait and retrieve results
        cur.get_results_from_sfqid(sfqid)

        # Fetch and verify results
        row = cur.fetchone()
        assert row is not None
        assert row[0] == 42
        assert row[1] == "hello"


def test_query_status_checking(connection):
    """Test query status checking methods."""
    with connection.cursor() as cur:
        # Execute async query
        result = cur.execute_async("SELECT COUNT(*) FROM table(generator(timeLimit => 2))")
        sfqid = result["queryId"]

        # Check status - should be running or completed
        status = connection.get_query_status(sfqid)
        assert isinstance(status, QueryStatus)

        # Wait for completion
        max_wait = 10  # seconds
        start_time = time.time()
        while connection.is_still_running(status) and (time.time() - start_time) < max_wait:
            time.sleep(0.5)
            status = connection.get_query_status(sfqid)

        # Should be successful
        assert status == QueryStatus.SUCCESS or not connection.is_an_error(status)


def test_long_running_query_with_polling(connection):
    """Test async execution with a longer-running query."""
    with connection.cursor() as cur:
        # Execute a query that takes a few seconds
        result = cur.execute_async("SELECT COUNT(*) FROM table(generator(timeLimit => 3))")
        sfqid = result["queryId"]

        # Poll until complete
        status = connection.get_query_status(sfqid)
        poll_count = 0
        while connection.is_still_running(status) and poll_count < 20:
            time.sleep(0.5)
            status = connection.get_query_status(sfqid)
            poll_count += 1

        # Get results
        cur.get_results_from_sfqid(sfqid)
        row = cur.fetchone()
        assert row is not None
        assert row[0] > 0


@pytest.mark.skip_reference
def test_execute_async_rejects_put_command(connection):
    """Test that PUT commands are rejected with execute_async."""
    with connection.cursor() as cur:
        with pytest.raises(NotSupportedError, match="PUT and GET statements are not supported"):
            cur.execute_async("PUT file:///tmp/test.txt @~")


@pytest.mark.skip_reference
def test_execute_async_rejects_get_command(connection):
    """Test that GET commands are rejected with execute_async."""
    with connection.cursor() as cur:
        with pytest.raises(NotSupportedError, match="PUT and GET statements are not supported"):
            cur.execute_async("GET @~/test.txt file:///tmp/")


@pytest.mark.skip_reference
def test_get_results_from_invalid_query_id(connection):
    """Test error handling for invalid query ID."""
    with connection.cursor() as cur:
        with pytest.raises(DatabaseError, match="Invalid query ID format"):
            cur.get_results_from_sfqid("not-a-valid-uuid")


def test_query_status_throw_if_error(connection):
    """Test get_query_status_throw_if_error method."""
    with connection.cursor() as cur:
        # Execute a valid query
        result = cur.execute_async("SELECT 1")
        sfqid = result["queryId"]

        # Wait for completion
        time.sleep(1)

        # Should not throw for successful query
        status = connection.get_query_status_throw_if_error(sfqid)
        assert status == QueryStatus.SUCCESS or not connection.is_an_error(status)


def test_is_still_running_helper(connection):
    """Test is_still_running helper method."""
    # Test various statuses
    assert connection.is_still_running(QueryStatus.RUNNING)
    assert connection.is_still_running(QueryStatus.QUEUED)
    assert connection.is_still_running(QueryStatus.RESUMING_WAREHOUSE)
    assert not connection.is_still_running(QueryStatus.SUCCESS)
    assert not connection.is_still_running(QueryStatus.FAILED_WITH_ERROR)


def test_is_an_error_helper(connection):
    """Test is_an_error helper method."""
    # Test various statuses
    assert connection.is_an_error(QueryStatus.FAILED_WITH_ERROR)
    assert connection.is_an_error(QueryStatus.ABORTED)
    assert connection.is_an_error(QueryStatus.FAILED_WITH_INCIDENT)
    assert not connection.is_an_error(QueryStatus.SUCCESS)
    assert not connection.is_an_error(QueryStatus.RUNNING)


def test_async_with_multiple_rows(connection):
    """Test async execution with multiple rows."""
    with connection.cursor() as cur:
        # Execute query that returns multiple rows
        result = cur.execute_async("SELECT seq4() AS n FROM table(generator(rowCount => 5))")
        sfqid = result["queryId"]

        # Get results
        cur.get_results_from_sfqid(sfqid)

        # Fetch all rows
        rows = cur.fetchall()
        assert len(rows) == 5
        assert all(isinstance(row[0], int) for row in rows)


def test_sfqid_property(connection):
    """Test that sfqid property is set correctly."""
    with connection.cursor() as cur:
        # Regular execute
        cur.execute("SELECT 1")
        sfqid_sync = cur.sfqid
        assert sfqid_sync is not None

        # Async execute
        result = cur.execute_async("SELECT 2")
        sfqid_async = cur.sfqid
        assert sfqid_async is not None
        assert sfqid_async == result["queryId"]
        assert sfqid_async != sfqid_sync  # Different queries should have different IDs
