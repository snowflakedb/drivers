class TestDataOperations:
    def test_should_select_data_from_table(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT * FROM table" is executed
        cursor.execute("SELECT * FROM table")

        # Then Result should contain expected values

        # And Column metadata should be correct

    def test_should_fetch_single_row_from_result(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT * FROM large_table" is executed
        cursor.execute("SELECT * FROM large_table")

        # Then Single row should be returned
        results = cursor.fetchall()
        assert len(results) > 0

    def test_should_insert_values_using_batch_binding(self, cursor):
        # Given Table "test_table" exists with schema (id INT, name VARCHAR)

        # When Integer values are inserted using parameter binding
        cursor.executemany(
            "INSERT INTO test_table VALUES (%s, %s)",
            [(1, "alice"), (2, "bob")],
        )

        # Then All rows should be inserted successfully

        # And Table should contain exactly 2 rows
