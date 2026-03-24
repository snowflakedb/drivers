"""
Integration tests for cursor.timestamp_output_format property.

This property exposes the session-level TIMESTAMP_OUTPUT_FORMAT parameter
as read-only metadata on the cursor. It is populated from the server
response parameters after query execution.
"""

import pytest


class TestTimestampOutputFormat:
    """Tests for cursor.timestamp_output_format property."""

    def test_set_at_connect_time(self, connection_factory):
        with connection_factory(session_parameters={"TIMESTAMP_OUTPUT_FORMAT": "YYYY-MM-DD"}) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert cursor.timestamp_output_format == "YYYY-MM-DD"

    def test_reflects_alter_session_change(self, cursor):
        cursor.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS'")
        assert cursor.timestamp_output_format == "YYYY-MM-DD HH24:MI:SS"

        cursor.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'MM/DD/YYYY HH24:MI'")
        assert cursor.timestamp_output_format == "MM/DD/YYYY HH24:MI"

    @pytest.mark.skip_reference(
        reason="UD uses shared connection cache; cursor_b sees update without executing a query"
    )
    def test_alter_session_visible_across_cursors_without_execute(self, connection):
        cursor_a = connection.cursor()
        cursor_b = connection.cursor()

        cursor_a.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY/MM/DD'")
        assert cursor_a.timestamp_output_format == "YYYY/MM/DD"
        assert cursor_b.timestamp_output_format == "YYYY/MM/DD"

    def test_alter_session_visible_across_cursors(self, connection):
        cursor_a = connection.cursor()
        cursor_b = connection.cursor()

        cursor_a.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY/MM/DD'")
        cursor_b.execute("SELECT 1")
        assert cursor_a.timestamp_output_format == "YYYY/MM/DD"
        assert cursor_b.timestamp_output_format == "YYYY/MM/DD"

    def test_alter_session_overrides_connect_time_value(self, connection_factory):
        with connection_factory(session_parameters={"TIMESTAMP_OUTPUT_FORMAT": "YYYY-MM-DD"}) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert cursor.timestamp_output_format == "YYYY-MM-DD"

            cursor.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'MM/DD/YYYY HH24:MI'")
            assert cursor.timestamp_output_format == "MM/DD/YYYY HH24:MI"

    def test_independent_connections_have_independent_formats(self, connection_factory):
        with (
            connection_factory(session_parameters={"TIMESTAMP_OUTPUT_FORMAT": "YYYY-MM-DD"}) as conn_a,
            connection_factory(session_parameters={"TIMESTAMP_OUTPUT_FORMAT": "MM/DD/YYYY"}) as conn_b,
        ):
            cursor_a = conn_a.cursor()
            cursor_b = conn_b.cursor()
            cursor_a.execute("SELECT 1")
            cursor_b.execute("SELECT 1")
            assert cursor_a.timestamp_output_format == "YYYY-MM-DD"
            assert cursor_b.timestamp_output_format == "MM/DD/YYYY"
