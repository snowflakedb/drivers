from datetime import datetime, timezone


class TestCriticalUserJourneys:
    def test_desc_command(self, cursor):
        # When DESC SCHEMA command is executed
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC';")
        rows = cursor.execute("desc schema snowflake.INFORMATION_SCHEMA").fetchall()

        # Then Schema properties are returned with correct types
        assert len(rows) > 0
        row = rows[0]
        created_on, name, kind = row[:3]
        assert isinstance(name, str)
        assert isinstance(kind, str)

        assert isinstance(created_on, datetime)
        assert created_on == datetime(1970, 1, 1, 0, 0, tzinfo=timezone.utc)

    def test_show_command(self, tmp_schema, cursor):
        # When SHOW SCHEMAS command is executed
        current_database = cursor.execute("select current_database()").fetchone()[0]
        r = cursor.execute(f"show schemas in database {current_database}").fetchall()

        # Then Result contains INFORMATION_SCHEMA and PUBLIC schemas
        schema_names = [row[1].upper() for row in r]
        assert "INFORMATION_SCHEMA" in schema_names
        assert "PUBLIC" in schema_names
