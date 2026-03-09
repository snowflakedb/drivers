class TestCriticalUserJourneys:
    def test_desc_command(self, tmp_schema, cursor):
        r = cursor.execute(f"desc schema {tmp_schema}").fetch_all()
        assert r == []

    def test_show_command(self, tmp_schema, cursor):
        current_database = cursor.execute("select current_database()").fetchone()[0]
        r = cursor.execute(f"show schemas in database {current_database}").fetchall()
        assert r == []
