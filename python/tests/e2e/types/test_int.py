"""INT type tests for Universal Driver.

This module tests INT type and its synonyms (INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT)
across various scenarios including literals, table operations, boundary values, NULL handling,
parameter binding, large result sets, and type casting.

All tests are parameterized to run with each type synonym to verify they behave identically.
"""

import pytest

from .utils import assert_sequential_values, assert_type


# =============================================================================
# TYPE SYNONYMS
# =============================================================================
# https://docs.snowflake.com/en/sql-reference/data-types-numeric
INT_TYPE_SYNONYMS = [
    "INT",
    "INTEGER",
    "BIGINT",
    "SMALLINT",
    "TINYINT",
    "BYTEINT",
]
int_type_parametrize = pytest.mark.parametrize("int_type", INT_TYPE_SYNONYMS)

# =============================================================================
# 8-BIT BOUNDARY VALUES
# =============================================================================
# Signed 8-bit boundaries (TINYINT range: -128 to 127)
INT8_SIGNED_MIN = -128
INT8_SIGNED_MAX = 127
# Unsigned 8-bit max (BYTEINT can store 0-255)
INT8_UNSIGNED_MAX = 255

# =============================================================================
# 16-BIT BOUNDARY VALUES
# =============================================================================
# Signed 16-bit boundaries (SMALLINT range: -32768 to 32767)
INT16_SIGNED_MIN = -32768
INT16_SIGNED_MAX = 32767
# Unsigned 16-bit max
INT16_UNSIGNED_MAX = 65535

# =============================================================================
# 32-BIT BOUNDARY VALUES
# =============================================================================
# Signed 32-bit boundaries (INT/INTEGER range: -2147483648 to 2147483647)
INT32_SIGNED_MIN = -2147483648
INT32_SIGNED_MAX = 2147483647
# Unsigned 32-bit max
INT32_UNSIGNED_MAX = 4294967295

# =============================================================================
# 64-BIT BOUNDARY VALUES
# =============================================================================
# Signed 64-bit boundaries (BIGINT range: -9223372036854775808 to 9223372036854775807)
INT64_SIGNED_MIN = -9223372036854775808
INT64_SIGNED_MAX = 9223372036854775807

# =============================================================================
# 38-DIGIT BOUNDARY VALUES (SNOWFLAKE MAXIMUM PRECISION)
# =============================================================================
# Snowflake NUMBER type supports up to 38 digits of precision
# These values trigger multi-byte internal representation
INT38_MAX = 99999999999999999999999999999999999999
INT38_MIN = -99999999999999999999999999999999999999

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 50_000


class TestIntTypeCasting:
    """Tests for INT type casting to appropriate type."""

    @int_type_parametrize
    def test_should_cast_integer_values_to_appropriate_type_for_int_and_synonyms(self, execute_query, int_type):
        # Given Snowflake client is logged in

        # When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
        sql = f"SELECT 0::{int_type}, 1000000::{int_type}, {INT64_SIGNED_MAX}::{int_type}"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type
        assert_type(result, int)

        # And No precision loss should occur
        assert result == (0, 1000000, INT64_SIGNED_MAX)


class TestIntLiteral:
    """Tests for INT type using SELECT with literals (no tables)."""

    @int_type_parametrize
    def test_should_select_integer_literals_for_int_and_synonyms(self, execute_query, int_type):
        # Given Snowflake client is logged in

        # When Query "SELECT 0::<type>, 1::<type>, -1::<type>, 42::<type>" is executed
        sql = f"SELECT 0::{int_type}, 1::{int_type}, -1::{int_type}, 42::{int_type}"
        result = execute_query(sql, single_row=True)

        # Then Result should contain integers [0, 1, -1, 42]
        assert result == (0, 1, -1, 42)
        assert_type(result, int)

    @int_type_parametrize
    def test_should_handle_integer_boundary_values_for_int_and_synonyms(self, execute_query, int_type):
        # Given Snowflake client is logged in

        # When Query "SELECT -128::<type>, 127::<type>, 255::<type>" is executed
        result = execute_query(
            f"SELECT {INT8_SIGNED_MIN}::{int_type}, {INT8_SIGNED_MAX}::{int_type}, {INT8_UNSIGNED_MAX}::{int_type}",
            single_row=True,
        )
        # Then Result should contain integers [-128, 127, 255]
        assert result == (INT8_SIGNED_MIN, INT8_SIGNED_MAX, INT8_UNSIGNED_MAX)
        assert_type(result, int)

        # When Query "SELECT -32768::<type>, 32767::<type>, 65535::<type>" is executed
        result = execute_query(
            f"SELECT {INT16_SIGNED_MIN}::{int_type}, {INT16_SIGNED_MAX}::{int_type}, {INT16_UNSIGNED_MAX}::{int_type}",
            single_row=True,
        )
        # Then Result should contain integers [-32768, 32767, 65535]
        assert result == (INT16_SIGNED_MIN, INT16_SIGNED_MAX, INT16_UNSIGNED_MAX)
        assert_type(result, int)

        # When Query "SELECT -2147483648::<type>, 2147483647::<type>, 4294967295::<type>" is executed
        result = execute_query(
            f"SELECT {INT32_SIGNED_MIN}::{int_type}, {INT32_SIGNED_MAX}::{int_type}, {INT32_UNSIGNED_MAX}::{int_type}",
            single_row=True,
        )
        # Then Result should contain integers [-2147483648, 2147483647, 4294967295]
        assert result == (INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT32_UNSIGNED_MAX)
        assert_type(result, int)

        # When Query "SELECT -9223372036854775808::<type>, 9223372036854775807::<type>" is executed
        result = execute_query(
            f"SELECT {INT64_SIGNED_MIN}::{int_type}, {INT64_SIGNED_MAX}::{int_type}",
            single_row=True,
        )
        # Then Result should contain integers [-9223372036854775808, 9223372036854775807]
        assert result == (INT64_SIGNED_MIN, INT64_SIGNED_MAX)
        assert_type(result, int)

    @int_type_parametrize
    def test_should_handle_large_integer_values_for_int_and_synonyms(self, execute_query, int_type):
        # Given Snowflake client is logged in

        # When Query "SELECT -99999999999999999999999999999999999999::<type>,
        #   99999999999999999999999999999999999999::<type>" is executed
        result = execute_query(f"SELECT {INT38_MIN}::{int_type}, {INT38_MAX}::{int_type}", single_row=True)

        # Then Result should contain integers [-99999999999999999999999999999999999999,
        #   99999999999999999999999999999999999999]
        assert result == (INT38_MIN, INT38_MAX)
        assert_type(result, int)

    @int_type_parametrize
    def test_should_handle_null_values_for_int_and_synonyms(self, execute_query, int_type):
        # Given Snowflake client is logged in

        # When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
        result = execute_query(
            f"SELECT NULL::{int_type}, 42::{int_type}, NULL::{int_type}",
            single_row=True,
        )

        # Then Result should contain [NULL, 42, NULL]
        assert result == (None, 42, None)
        assert_type(result, int, can_be_none=True)

    @int_type_parametrize
    def test_should_download_large_result_set_with_multiple_chunks_for_int_and_synonyms(self, execute_query, int_type):
        # Given Snowflake client is logged in

        # When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id" is executed

        # Note: seq8() doesn't guarantee consecutive values in parallel execution,
        # so we use ROW_NUMBER() to ensure sequential integers.
        sql = (
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::{int_type} as id "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY 1"
        )
        rows = execute_query(sql)

        # Then Result should contain 50000 sequentially numbered rows from 0 to 49999
        values = [row[0] for row in rows]
        assert_type(values, int)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE)


class TestIntTable:
    """Tests for INT type using table operations."""

    @int_type_parametrize
    def test_should_select_integers_from_table_for_int_and_synonyms(self, execute_query, tmp_schema, int_type):
        # Given Snowflake client is logged in

        # And Table with <type> column exists with values [0, 1, -1, 100]
        table_name = f"{tmp_schema}.int_table_{int_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col {int_type})")
        test_values = [0, 1, -1, 100]
        for val in test_values:
            execute_query(f"INSERT INTO {table_name} VALUES ({val})")

        # When Query "SELECT * FROM int_table ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain integers [-1, 0, 1, 100]
        result = [row[0] for row in rows]
        assert result == [-1, 0, 1, 100]
        assert_type(result, int)

    def test_should_select_corner_case_values_from_table_for_int_and_synonyms(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in

        # And Table with columns (tinyint_col TINYINT, byteint_col BYTEINT, smallint_col SMALLINT,
        # int_col INT, integer_col INTEGER, bigint_col BIGINT, int38_col INT) exists
        table_name = f"{tmp_schema}.corner_case_table"
        execute_query(
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
            INT8_SIGNED_MAX,
            INT8_UNSIGNED_MAX,
            INT16_SIGNED_MAX,
            INT32_SIGNED_MAX,
            INT32_SIGNED_MAX,
            INT64_SIGNED_MAX,
            INT38_MAX,
        )
        execute_query(
            f"INSERT INTO {table_name} VALUES ({positive_row[0]}, {positive_row[1]}, {positive_row[2]}, "
            f"{positive_row[3]}, {positive_row[4]}, {positive_row[5]}, {positive_row[6]})"
        )

        # And Row with negative values (-128, -1, -32768, -2147483648, -2147483648, -9223372036854775808,
        # -99999999999999999999999999999999999999) is inserted
        negative_row = (
            INT8_SIGNED_MIN,
            -1,
            INT16_SIGNED_MIN,
            INT32_SIGNED_MIN,
            INT32_SIGNED_MIN,
            INT64_SIGNED_MIN,
            INT38_MIN,
        )
        execute_query(
            f"INSERT INTO {table_name} VALUES ({negative_row[0]}, {negative_row[1]}, {negative_row[2]}, "
            f"{negative_row[3]}, {negative_row[4]}, {negative_row[5]}, {negative_row[6]})"
        )

        # And Row with zeroes and nulls (0, NULL, 0, NULL, 0, NULL, 0) is inserted
        zeroes_nulls_row = (0, None, 0, None, 0, None, 0)
        execute_query(f"INSERT INTO {table_name} VALUES (0, NULL, 0, NULL, 0, NULL, 0)")

        # When Query "SELECT * FROM corner_case_table" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain 3 rows with expected corner case values for all int type synonyms
        assert len(rows) == 3, f"Expected 3 rows, got {len(rows)}"

        # Verify positive row
        assert rows[0] == positive_row
        assert_type(rows[0], int)

        # Verify negative row
        assert rows[1] == negative_row
        assert_type(rows[1], int)

        # Verify zeroes and nulls row
        assert rows[2] == zeroes_nulls_row
        assert_type(rows[2], int, can_be_none=True)

    @int_type_parametrize
    def test_should_select_large_result_set_from_table_for_int_and_synonyms(self, execute_query, tmp_schema, int_type):
        # Given Snowflake client is logged in

        # And Table with <type> column exists with 50000 sequential values

        # Note: seq8() doesn't guarantee consecutive values in parallel execution,
        # so we use ROW_NUMBER() to ensure sequential integers.
        table_name = f"{tmp_schema}.large_int_table_{int_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col {int_type})")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::{int_type} "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain 50000 sequentially numbered rows from 0 to 49999
        values = [row[0] for row in rows]
        assert_type(values, int)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE)


@pytest.mark.skip_reference
class TestIntBinding:
    """Tests for INT type using parameter binding."""

    @int_type_parametrize
    def test_should_insert_integer_using_parameter_binding_for_int_and_synonyms(
        self, execute_query, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.int_bind_table_{int_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col {int_type})")

        # When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
        test_values = [0, INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT64_SIGNED_MAX]
        for val in test_values:
            execute_query(f"INSERT INTO {table_name} VALUES (?)", (val,))

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain integers [0, -2147483648, 2147483647, 9223372036854775807]
        result = [row[0] for row in rows]
        assert result == test_values
        assert_type(result, int)

    @int_type_parametrize
    def test_should_insert_and_select_integers_from_table_using_parameter_binding_for_int_and_synonyms(
        self, execute_query, executemany_insert, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.int_bind_table_{int_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col {int_type})")

        # When Integer values [0, 42, -2147483648, 9223372036854775807] are inserted using binding
        test_values = [0, 42, INT32_SIGNED_MIN, INT64_SIGNED_MAX]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", [(val,) for val in test_values])

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain integers [0, 42, -2147483648, 9223372036854775807]
        result = sorted([row[0] for row in rows])
        expected = sorted(test_values)
        assert result == expected
        assert_type(result, int)
