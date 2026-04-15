"""Connection property tests for Universal Driver (Python).

This module tests connection properties that require a live Snowflake session:
- snowflake_version returns a valid server version string
"""

from __future__ import annotations

import re


class TestConnectionProperties:
    """Tests for connection properties."""

    def test_should_return_valid_snowflake_version_string(self, connection):
        """Test that snowflake_version returns a semver-like version string."""
        # Given Snowflake client is logged in
        pass

        # When The snowflake_version property is accessed
        version = connection.snowflake_version

        # Then the version should be a non-empty string matching a version pattern
        assert isinstance(version, str)
        assert re.match(r"^\d+\.\d+\.\d+", version), f"Unexpected version format: {version!r}"

    def test_should_match_current_version_query(self, connection, cursor):
        """Test that snowflake_version matches SELECT CURRENT_VERSION()."""
        # Given Snowflake client is logged in
        pass

        # When The snowflake_version property is accessed
        property_version = connection.snowflake_version

        # And CURRENT_VERSION() is queried directly
        cursor.execute("SELECT CURRENT_VERSION()")
        query_version = str(cursor.fetchone()[0]).split(" ")[0]

        # Then the property value should match the direct query result
        assert property_version == query_version

    def test_should_return_cached_result(self, connection):
        """Test that snowflake_version returns the same cached value on repeated access."""
        # Given Snowflake client is logged in
        pass

        # When The snowflake_version property is accessed twice
        first = connection.snowflake_version
        second = connection.snowflake_version

        # Then both accesses should return the same value
        assert first == second
