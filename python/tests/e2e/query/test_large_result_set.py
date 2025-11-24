import pytest


class TestLargeResultSet:
    
    def test_should_process_one_million_row_result_set(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"
        
        # When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
        sql = "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id"
        cursor.execute(sql)
        rows = cursor.fetchall()
        
        # Then there are 1000000 numbered sequentially rows returned
        expected_count = 1000000
        assert len(rows) == expected_count
        for i, row in enumerate(rows):
            assert row[0] == i
