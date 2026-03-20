from tests.e2e.types.utils import assert_sequential_values


class TestLargeResultSet:
    def test_should_process_one_million_row_result_set(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed

        # Note: We use ROW_NUMBER() for actual query to ensure sequential integers.
        sql = (
            "SELECT ROW_NUMBER() OVER (ORDER BY seq8()) - 1 as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) ORDER BY 1"
        )
        cursor.execute(sql)
        rows = cursor.fetchall()

        # Then there are 1000000 numbered sequentially rows returned
        values = [row[0] for row in rows]
        assert_sequential_values(values, 1000000)
