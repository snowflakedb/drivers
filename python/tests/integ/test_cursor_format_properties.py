"""
Integration tests for cursor session format properties.

These properties expose session-level output format parameters
as read-only metadata on the cursor.
"""

import pytest


SIMPLE_PROPERTIES = [
    ("timestamp_output_format", "YYYY-MM-DD", "MM/DD/YYYY HH24:MI"),
    ("date_output_format", "YYYY-MM-DD", "DD/MM/YYYY"),
    ("time_output_format", "HH24:MI:SS", "HH12:MI:SS"),
    ("timezone", "America/New_York", "Europe/Berlin"),
    ("binary_output_format", "HEX", "BASE64"),
]


class TestSessionFormatProperties:
    """Tests for simple session format properties (no fallback logic).

    Covers: timestamp_output_format, date_output_format, time_output_format,
    timezone, and binary_output_format.
    """

    @pytest.mark.parametrize("prop,val1,val2", SIMPLE_PROPERTIES)
    def test_set_at_connect_time(self, connection_factory, prop, val1, val2):
        with connection_factory(session_parameters={prop.upper(): val1}) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert getattr(cursor, prop) == val1

    @pytest.mark.parametrize("prop,val1,val2", SIMPLE_PROPERTIES)
    def test_reflects_alter_session_change(self, cursor, prop, val1, val2):
        cursor.execute(f"ALTER SESSION SET {prop.upper()} = '{val1}'")
        assert getattr(cursor, prop) == val1

        cursor.execute(f"ALTER SESSION SET {prop.upper()} = '{val2}'")
        assert getattr(cursor, prop) == val2

    @pytest.mark.parametrize("prop,val1,val2", SIMPLE_PROPERTIES)
    def test_alter_session_overrides_connect_time_value(self, connection_factory, prop, val1, val2):
        with connection_factory(session_parameters={prop.upper(): val1}) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert getattr(cursor, prop) == val1

            cursor.execute(f"ALTER SESSION SET {prop.upper()} = '{val2}'")
            assert getattr(cursor, prop) == val2

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


TIMESTAMP_FALLBACK_PROPERTIES = [
    "timestamp_ltz_output_format",
    "timestamp_tz_output_format",
    "timestamp_ntz_output_format",
]


class TestTimestampTypeSpecificOutputFormats:
    """Tests for cursor.timestamp_{ltz,tz,ntz}_output_format properties.

    These properties fall back to timestamp_output_format when not set explicitly.
    """

    @pytest.mark.parametrize("prop", TIMESTAMP_FALLBACK_PROPERTIES)
    def test_falls_back_to_timestamp_output_format(self, cursor, prop):
        cursor.execute(f"ALTER SESSION SET {prop.upper()} = ''")
        cursor.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS'")
        assert getattr(cursor, prop) == "YYYY-MM-DD HH24:MI:SS"

    @pytest.mark.parametrize("prop", TIMESTAMP_FALLBACK_PROPERTIES)
    def test_explicit_value_overrides_fallback(self, cursor, prop):
        cursor.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS'")
        cursor.execute(f"ALTER SESSION SET {prop.upper()} = 'DD/MM/YYYY'")
        assert getattr(cursor, prop) == "DD/MM/YYYY"

    @pytest.mark.parametrize("prop", TIMESTAMP_FALLBACK_PROPERTIES)
    def test_set_at_connect_time(self, connection_factory, prop):
        with connection_factory(session_parameters={prop.upper(): "YYYY/MM/DD HH24:MI"}) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert getattr(cursor, prop) == "YYYY/MM/DD HH24:MI"

    @pytest.mark.parametrize("prop", TIMESTAMP_FALLBACK_PROPERTIES)
    def test_alter_session_overrides_connect_time_value(self, connection_factory, prop):
        with connection_factory(session_parameters={prop.upper(): "YYYY-MM-DD"}) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT 1")
            assert getattr(cursor, prop) == "YYYY-MM-DD"

            cursor.execute(f"ALTER SESSION SET {prop.upper()} = 'DD.MM.YYYY HH24:MI'")
            assert getattr(cursor, prop) == "DD.MM.YYYY HH24:MI"
