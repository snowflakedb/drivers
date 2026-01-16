"""INT type tests for Universal Driver.

This module tests INT type and its synonyms (INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT)
across various scenarios including literals, table operations, boundary values, NULL handling,
parameter binding, large result sets, and type casting.

All tests are parameterized to run with each type synonym to verify they behave identically.
"""

import pytest


class TestInt:
    """Test suite for INT type and synonyms."""

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_select_integer_literals_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 0::<type>, 1::<type>, -1::<type>, 42::<type>" is executed
        sql = f"SELECT 0::{int_type}, 1::{int_type}, -1::{int_type}, 42::{int_type}"
        cursor.execute(sql)
        result = cursor.fetchone()

        # Then Result should contain integers [0, 1, -1, 42]
        assert result == (0, 1, -1, 42), f"Expected (0, 1, -1, 42), got {result}"
        assert all(isinstance(val, int) for val in result), "All values should be Python int type"

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_select_integers_from_table_for_int_and_synonyms(self, cursor, tmp_schema, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists with values [0, 1, -1, 100]
        table_name = f"{tmp_schema}.int_table_{int_type.lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {int_type})")

        test_values = [0, 1, -1, 100]
        for val in test_values:
            cursor.execute(f"INSERT INTO {table_name} VALUES ({val})")

        # When Query "SELECT * FROM int_table ORDER BY col" is executed
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY col")
        rows = cursor.fetchall()

        # Then Result should contain integers [-1, 0, 1, 100]
        result = [row[0] for row in rows]
        expected = [-1, 0, 1, 100]
        assert result == expected, f"Expected {expected}, got {result}"
        assert all(isinstance(val, int) for val in result), "All values should be Python int type"

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_handle_integer_boundary_values_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT -128::<type>, 127::<type>, 255::<type>" is executed
        cursor.execute(f"SELECT -128::{int_type}, 127::{int_type}, 255::{int_type}")
        result = cursor.fetchone()
        # Then Result should contain integers [-128, 127, 255]
        assert result == (
            -128,
            127,
            255,
        ), f"8-bit: Expected (-128, 127, 255), got {result}"
        assert all(isinstance(val, int) for val in result)

        # When Query "SELECT -32768::<type>, 32767::<type>, 65535::<type>" is executed
        cursor.execute(f"SELECT -32768::{int_type}, 32767::{int_type}, 65535::{int_type}")
        result = cursor.fetchone()
        # Then Result should contain integers [-32768, 32767, 65535]
        assert result == (
            -32768,
            32767,
            65535,
        ), f"16-bit: Expected (-32768, 32767, 65535), got {result}"
        assert all(isinstance(val, int) for val in result)

        # When Query "SELECT -2147483648::<type>, 2147483647::<type>, 4294967295::<type>" is executed
        cursor.execute(f"SELECT -2147483648::{int_type}, 2147483647::{int_type}, 4294967295::{int_type}")
        result = cursor.fetchone()
        # Then Result should contain integers [-2147483648, 2147483647, 4294967295]
        assert result == (
            -2147483648,
            2147483647,
            4294967295,
        ), f"32-bit: Expected (-2147483648, 2147483647, 4294967295), got {result}"
        assert all(isinstance(val, int) for val in result)

        # When Query "SELECT -9223372036854775808::<type>, 9223372036854775807::<type>" is executed
        cursor.execute(f"SELECT -9223372036854775808::{int_type}, 9223372036854775807::{int_type}")
        result = cursor.fetchone()
        # Then Result should contain integers [-9223372036854775808, 9223372036854775807]
        assert result == (
            -9223372036854775808,
            9223372036854775807,
        ), f"64-bit: Expected (-9223372036854775808, 9223372036854775807), got {result}"
        assert all(isinstance(val, int) for val in result)

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_handle_large_integer_values_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT -99999999999999999999999999999999999999::<type>,
        #   99999999999999999999999999999999999999::<type>" is executed
        min_38_digit = -99999999999999999999999999999999999999
        max_38_digit = 99999999999999999999999999999999999999
        cursor.execute(f"SELECT {min_38_digit}::{int_type}, {max_38_digit}::{int_type}")
        result = cursor.fetchone()

        # Then Result should contain integers [-99999999999999999999999999999999999999,
        #   99999999999999999999999999999999999999]
        assert result == (
            min_38_digit,
            max_38_digit,
        ), f"38-digit: Expected ({min_38_digit}, {max_38_digit}), got {result}"
        assert all(isinstance(val, int) for val in result), (
            "Python int supports arbitrary precision and should handle 38-digit integers"
        )

    def test_should_select_corner_case_values_from_table_for_int_and_synonyms(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with columns (tinyint_col TINYINT, byteint_col BYTEINT, smallint_col SMALLINT,
        # int_col INT, integer_col INTEGER, bigint_col BIGINT, int38_col INT) exists
        table_name = f"{tmp_schema}.corner_case_table"
        # weird definition format as tests validator is getting a stroke trying to read multiline strings
        cursor.execute(
            f"CREATE TABLE {table_name} ("
            "tinyint_col TINYINT, "
            "byteint_col BYTEINT, "
            "smallint_col SMALLINT, "
            "int_col INT, "
            "integer_col INTEGER, "
            "bigint_col BIGINT, "
            "int38_col INT)"
        )

        # And Row with positive values (127, 255, 32767, 2147483647, 2147483647, 9223372036854775807,
        # 99999999999999999999999999999999999999) is inserted
        positive_row = (
            127,  # 8-bit signed max
            255,  # 8-bit unsigned max
            32767,  # 16-bit signed max
            2147483647,  # 32-bit signed max
            2147483647,  # 32-bit signed max
            9223372036854775807,  # 64-bit signed max
            99999999999999999999999999999999999999,  # 38-digit max
        )
        cursor.execute(
            f"INSERT INTO {table_name} VALUES ({positive_row[0]}, {positive_row[1]}, {positive_row[2]}, "
            f"{positive_row[3]}, {positive_row[4]}, {positive_row[5]}, {positive_row[6]})"
        )

        # And Row with negative values (-128, -1, -32768, -2147483648, -2147483648, -9223372036854775808,
        # -99999999999999999999999999999999999999) is inserted
        negative_row = (
            -128,  # 8-bit signed min
            -1,  # simple negative
            -32768,  # 16-bit signed min
            -2147483648,  # 32-bit signed min
            -2147483648,  # 32-bit signed min
            -9223372036854775808,  # 64-bit signed min
            -99999999999999999999999999999999999999,  # 38-digit min
        )
        cursor.execute(
            f"INSERT INTO {table_name} VALUES ({negative_row[0]}, {negative_row[1]}, {negative_row[2]}, "
            f"{negative_row[3]}, {negative_row[4]}, {negative_row[5]}, {negative_row[6]})"
        )

        # And Row with zeroes and nulls (0, NULL, 0, NULL, 0, NULL, 0) is inserted
        zeroes_nulls_row = (0, None, 0, None, 0, None, 0)
        cursor.execute(f"INSERT INTO {table_name} VALUES (0, NULL, 0, NULL, 0, NULL, 0)")

        # When Query "SELECT * FROM corner_case_table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()

        # Then Result should contain 3 rows with expected corner case values for all int type synonyms
        assert len(rows) == 3, f"Expected 3 rows, got {len(rows)}"

        # Verify positive row
        assert rows[0] == positive_row, f"Positive row: Expected {positive_row}, got {rows[0]}"
        for val in rows[0]:
            assert isinstance(val, int), f"Value {val} in positive row should be Python int type"

        # Verify negative row
        assert rows[1] == negative_row, f"Negative row: Expected {negative_row}, got {rows[1]}"
        for val in rows[1]:
            assert isinstance(val, int), f"Value {val} in negative row should be Python int type"

        # Verify zeroes and nulls row
        assert rows[2] == zeroes_nulls_row, f"Zeroes/nulls row: Expected {zeroes_nulls_row}, got {rows[2]}"
        for i, val in enumerate(rows[2]):
            if val is not None:
                assert isinstance(val, int), f"Value {val} at index {i} should be Python int type"
            else:
                assert val is None, f"Value at index {i} should be NULL (None)"

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_handle_null_values_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
        cursor.execute(f"SELECT NULL::{int_type}, 42::{int_type}, NULL::{int_type}")
        result = cursor.fetchone()

        # Then Result should contain [NULL, 42, NULL]
        assert result == (None, 42, None), f"Expected (None, 42, None), got {result}"
        assert result[0] is None, "First value should be NULL (None)"
        assert isinstance(result[1], int), "Second value should be Python int"
        assert result[2] is None, "Third value should be NULL (None)"

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_insert_integer_using_parameter_binding_for_int_and_synonyms(self, cursor, tmp_schema, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.int_bind_table_{int_type.lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {int_type})")

        # When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
        test_values = [0, -2147483648, 2147483647, 9223372036854775807]
        for val in test_values:
            cursor.execute(f"INSERT INTO {table_name} VALUES (?)", (val,))

        # Then SELECT should return the same values in order
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()

        result = [row[0] for row in rows]
        assert result == test_values, f"Expected {test_values}, got {result}"
        assert all(isinstance(val, int) for val in result), "All values should be Python int type after round-trip"

    @pytest.mark.skip("SNOW-3006013 - parameter binding is not yet implemented")
    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_insert_and_select_integers_from_table_using_parameter_binding_for_int_and_synonyms(
        self, cursor, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.int_bind_table_{int_type.lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {int_type})")

        # When Integer values [0, 42, -2147483648, 9223372036854775807] are inserted using binding
        test_values = [0, 42, -2147483648, 9223372036854775807]
        for val in test_values:
            cursor.execute(f"INSERT INTO {table_name} VALUES (?)", (val,))

        # And Query "SELECT * FROM int_table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        rows = cursor.fetchall()

        # Then Result should contain integers [0, 42, -2147483648, 9223372036854775807]
        result = [row[0] for row in rows]
        assert result == test_values, f"Expected {test_values}, got {result}"
        assert all(isinstance(val, int) for val in result), "All values should be Python int type"

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_download_large_result_set_with_multiple_chunks_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
        sql = f"SELECT seq8()::{int_type} as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id"
        cursor.execute(sql)
        rows = cursor.fetchall()

        # Then Result should contain 1000000 sequentially numbered rows from 0 to 999999
        expected_count = 1000000
        assert len(rows) == expected_count, f"Expected {expected_count} rows, got {len(rows)}"

        # Verify sequential values (0 to 999999)
        for i, row in enumerate(rows):
            assert row[0] == i, f"Expected row {i} to have value {i}, got {row[0]}"
            assert isinstance(row[0], int), f"Value at row {i} should be Python int type"

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_select_large_result_set_from_table_for_int_and_synonyms(self, cursor, tmp_schema, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # And Table with <type> column exists with 1000000 sequential values
        table_name = f"{tmp_schema}.large_int_table_{int_type.lower()}"
        cursor.execute(f"CREATE TABLE {table_name} (col {int_type})")
        cursor.execute(f"INSERT INTO {table_name} SELECT seq8()::{int_type} FROM TABLE(GENERATOR(ROWCOUNT => 1000000))")

        # When Query "SELECT * FROM large_int_table ORDER BY col" is executed
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY col")
        rows = cursor.fetchall()

        # Then Result should contain 1000000 sequentially numbered rows from 0 to 999999
        expected_count = 1000000
        assert len(rows) == expected_count, f"Expected {expected_count} rows, got {len(rows)}"

        # Verify sequential values (0 to 999999)
        for i, row in enumerate(rows):
            assert row[0] == i, f"Expected row {i} to have value {i}, got {row[0]}"
            assert isinstance(row[0], int), f"Value at row {i} should be Python int type"

    @pytest.mark.parametrize("int_type", ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"])
    def test_should_cast_integer_values_to_appropiate_type_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
        cursor.execute(f"SELECT 0::{int_type}, 1000000::{int_type}, 9223372036854775807::{int_type}")
        result = cursor.fetchone()

        # Then All values should be returned as an appropiate type
        assert result == (
            0,
            1000000,
            9223372036854775807,
        ), f"Expected (0, 1000000, 9223372036854775807), got {result}"

        # And No precision loss should occur
        assert all(isinstance(val, int) for val in result), "All values should be cast to Python int type"
        assert result[0] == 0, "Small integer (0) should preserve exact value"
        assert result[1] == 1000000, "Medium integer (1000000) should preserve exact value"
        assert result[2] == 9223372036854775807, (
            "Large 64-bit integer should preserve exact value (Python int has arbitrary precision)"
        )
