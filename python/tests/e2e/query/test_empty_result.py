import pytest


class TestEmptyResult:

    def test_should_return_empty_result_when_query_produces_no_rows(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 1 WHERE FALSE" is executed
        cursor.execute("SELECT 1 WHERE FALSE")
        rows = cursor.fetchall()

        # Then empty result set is returned
        assert rows == []
