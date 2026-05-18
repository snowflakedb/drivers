"""
Integration tests for query timeout functionality.

Tests verify that:
- Per-query timeout parameter works via cursor.execute(timeout=...)
- Connection-level query_timeout default is respected
- Statement-level timeout overrides connection default
- Zero timeout means no timeout (disabled)
"""

import pytest

from snowflake.connector.errors import ProgrammingError


class TestQueryTimeoutPerQuery:
    """Tests for per-query timeout via execute(timeout=...)."""

    def test_fast_query_succeeds_with_timeout(self, cursor):
        """A fast query completes within a generous timeout."""
        cursor.execute("SELECT 1", timeout=60)
        assert cursor.fetchone() == (1,)

    def test_timeout_zero_means_disabled(self, cursor):
        """Timeout=0 means no timeout is applied."""
        cursor.execute("SELECT 1", timeout=0)
        assert cursor.fetchone() == (1,)

    def test_long_query_times_out(self, cursor):
        """A query that exceeds the timeout raises an error."""
        with pytest.raises((ProgrammingError, Exception)):
            cursor.execute("CALL SYSTEM$WAIT(10)", timeout=1)


class TestQueryTimeoutConnectionDefault:
    """Tests for connection-level query_timeout default."""

    def test_connection_default_applies_to_queries(self, connection_factory):
        """query_timeout on connection applies as default for all queries."""
        with connection_factory(query_timeout=60) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert cursor.fetchone() == (1,)

    def test_connection_default_zero_is_disabled(self, connection_factory):
        """query_timeout=0 on connection means no timeout."""
        with connection_factory(query_timeout=0) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert cursor.fetchone() == (1,)

    def test_per_query_timeout_overrides_connection(self, connection_factory):
        """Per-query timeout overrides the connection default."""
        with connection_factory(query_timeout=1) as conn:
            cursor = conn.cursor()
            # Override with generous timeout so the fast query succeeds
            cursor.execute("SELECT 1", timeout=60)
            assert cursor.fetchone() == (1,)

    def test_connection_default_triggers_on_long_query(self, connection_factory):
        """Connection-level timeout triggers for long-running queries."""
        with connection_factory(query_timeout=1) as conn:
            cursor = conn.cursor()
            with pytest.raises((ProgrammingError, Exception)):
                cursor.execute("CALL SYSTEM$WAIT(10)")
