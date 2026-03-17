from datetime import datetime
from zoneinfo import ZoneInfo


class TestCriticalUserJourneys:
    def test_desc_command(self, cursor):
        # When DESC SCHEMA command is executed
        la_tz = ZoneInfo("America/Los_Angeles")
        cursor.execute("ALTER SESSION SET TIMEZONE = 'America/Los_Angeles';")
        rows = cursor.execute("desc schema snowflake.INFORMATION_SCHEMA").fetchall()

        # Then Schema properties are returned with correct types
        assert len(rows) > 0
        row = rows[0]
        created_on, name, kind = row[:3]
        assert isinstance(name, str)
        assert isinstance(kind, str)

        assert isinstance(created_on, datetime)
        assert created_on == datetime(1969, 12, 31, 16, 0, tzinfo=la_tz)

    def test_show_command(self, tmp_schema, cursor):
        # When SHOW SCHEMAS command is executed
        r = cursor.execute("SHOW SCHEMAS").fetchall()

        # Then Result contains INFORMATION_SCHEMA and PUBLIC schemas
        schema_names = [row[1].upper() for row in r]
        assert "INFORMATION_SCHEMA" in schema_names
        assert "PUBLIC" in schema_names
        filtered = cursor.execute(f"SHOW SCHEMAS LIKE '{tmp_schema}'").fetchall()
        assert len(filtered) == 1
        assert filtered[0][1].upper() == tmp_schema.upper()
