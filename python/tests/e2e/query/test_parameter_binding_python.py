"""Python-specific parameter binding tests.

Tests for backward compatibility with Python-specific parameter formats
(tuple vs list).
"""

from __future__ import annotations

from ...conftest import with_paramstyle


@with_paramstyle("qmark")
class TestBackwardCompatibility:
    """Tests for backward compatibility with old connector parameter format."""

    def test_should_handle_both_tuple_and_list_parameter_formats(self, cursor):
        # When Query "SELECT ?, ?" is executed with tuple parameters (1, "test")
        sql = "SELECT ?, ?"
        cursor.execute(sql, (1, "test"))
        result_tuple = cursor.fetchone()

        # And Query "SELECT ?, ?" is executed with list parameters [1, "test"]
        cursor.execute(sql, [1, "test"])
        result_list = cursor.fetchone()

        # Then Both results should be identical
        assert result_tuple == result_list == (1, "test")
