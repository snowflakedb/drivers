"""INT type tests for Universal Driver.

This module tests INT type and its synonyms (INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT)
across various scenarios including literals, table operations, boundary values, NULL handling,
parameter binding, large result sets, and type casting.
"""

import pytest

from ...conftest import with_paramstyle
from .utils import assert_sequential_values, assert_type, batch_insert


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
        pass

        # When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
        sql = f"SELECT 0::{int_type}, 1000000::{int_type}, {INT64_SIGNED_MAX}::{int_type}"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate type with no precision loss
        assert_type(result, int)
        assert result == (0, 1000000, INT64_SIGNED_MAX)


class TestIntLiteral:
    """Tests for INT type using SELECT with literals (no tables)."""

    LITERAL_SELECT_TEST_CASES = [
        ("zero", [0], (0,)),
        (
            "tinyint",
            [INT8_SIGNED_MIN, INT8_SIGNED_MAX, INT8_UNSIGNED_MAX],
            (INT8_SIGNED_MIN, INT8_SIGNED_MAX, INT8_UNSIGNED_MAX),
        ),
        (
            "smallint",
            [INT16_SIGNED_MIN, INT16_SIGNED_MAX, INT16_UNSIGNED_MAX],
            (INT16_SIGNED_MIN, INT16_SIGNED_MAX, INT16_UNSIGNED_MAX),
        ),
        (
            "int",
            [INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT32_UNSIGNED_MAX],
            (INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT32_UNSIGNED_MAX),
        ),
        ("bigint", [INT64_SIGNED_MIN, INT64_SIGNED_MAX], (INT64_SIGNED_MIN, INT64_SIGNED_MAX)),
    ]

    @int_type_parametrize
    @pytest.mark.parametrize(
        "values,query_values,expected_values",
        LITERAL_SELECT_TEST_CASES,
        ids=[c[0] for c in LITERAL_SELECT_TEST_CASES],
    )
    def test_should_select_integer_values_for_int_and_synonyms(
        self, execute_query, int_type, values, query_values, expected_values
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"{v}::{int_type}" for v in query_values)
        result = execute_query(f"SELECT {select_cols}", single_row=True)

        # Then Result should contain integers <expected_values>
        assert result == expected_values
        assert_type(result, int)

    @int_type_parametrize
    def test_should_handle_large_integer_values_for_int_and_synonyms(self, execute_query, int_type):
        # Given Snowflake client is logged in
        pass

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
        pass

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
        pass

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

    TABLE_SELECT_TEST_CASES = [
        (
            "positive",
            [
                0,
                1,
                INT8_SIGNED_MAX,
                INT8_UNSIGNED_MAX,
                INT16_SIGNED_MAX,
                INT16_UNSIGNED_MAX,
                INT32_SIGNED_MAX,
                INT32_UNSIGNED_MAX,
                INT64_SIGNED_MAX,
            ],
            [
                0,
                1,
                INT8_SIGNED_MAX,
                INT8_UNSIGNED_MAX,
                INT16_SIGNED_MAX,
                INT16_UNSIGNED_MAX,
                INT32_SIGNED_MAX,
                INT32_UNSIGNED_MAX,
                INT64_SIGNED_MAX,
            ],
            False,
        ),
        (
            "negative",
            [-1, INT8_SIGNED_MIN, INT16_SIGNED_MIN, INT32_SIGNED_MIN, INT64_SIGNED_MIN],
            [INT64_SIGNED_MIN, INT32_SIGNED_MIN, INT16_SIGNED_MIN, INT8_SIGNED_MIN, -1],
            False,
        ),
        (
            "null",
            [0, None, 42],
            [0, 42, None],
            True,
        ),
    ]

    @int_type_parametrize
    @pytest.mark.parametrize(
        "values,insert_values,expected_values,can_be_none",
        TABLE_SELECT_TEST_CASES,
        ids=[c[0] for c in TABLE_SELECT_TEST_CASES],
    )
    def test_should_select_values_from_table_for_int_and_synonyms(
        self, execute_query, tmp_schema, int_type, values, insert_values, expected_values, can_be_none
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with values <insert_values>
        table_name = f"{tmp_schema}.int_table_{int_type.lower()}_{values}"
        execute_query(f"CREATE TABLE {table_name} (col {int_type})")
        batch_insert(execute_query, table_name, insert_values)

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
        result = [row[0] for row in rows]

        # Then Result should contain integers <expected_values>
        assert result == expected_values
        assert_type(result, int, can_be_none=can_be_none)

    @int_type_parametrize
    def test_should_select_large_integer_values_from_table_for_int_and_synonyms(
        self, execute_query, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with values
        # [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        table_name = f"{tmp_schema}.int38_table_{int_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col {int_type})")
        batch_insert(execute_query, table_name, [INT38_MIN, INT38_MAX])

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")
        result = [row[0] for row in rows]

        # Then Result should contain integers
        # [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        assert result == [INT38_MIN, INT38_MAX]
        assert_type(result, int)

    def test_should_handle_server_side_arrow_memory_optimization_for_int_columns_on_multiple_chunks(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with four INT columns exists
        table_name = f"{tmp_schema}.different_int_column_sizes"
        execute_query(f"CREATE TABLE {table_name} (col_int8 INT, col_int16 INT, col_int32 INT, col_int64 INT)")

        # And Each column contains values of different magnitudes (50000 rows to span multiple Arrow chunks)
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT 100, 30000, 2000000000, 9000000000000000000 "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain 50000 rows with all values equal to expected data
        assert len(rows) == LARGE_RESULT_SET_SIZE

        for row in rows:
            assert row == (100, 30000, 2000000000, 9000000000000000000)


@with_paramstyle("qmark")
class TestIntBinding:
    """Tests for INT type using parameter binding."""

    @int_type_parametrize
    def test_should_insert_integer_using_parameter_binding_for_int_and_synonyms(
        self, execute_query, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in
        pass

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
        result = sorted([row[0] for row in rows])
        expected = sorted(test_values)
        assert result == expected
        assert_type(result, int)

    @int_type_parametrize
    def test_should_insert_and_select_integers_from_table_using_batch_parameter_binding_for_int_and_synonyms(
        self, execute_query, executemany_insert, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.int_bind_table_{int_type.lower()}"
        execute_query(f"CREATE TABLE {table_name} (col {int_type})")

        # When Integer values [0, 42, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
        test_values = [0, 42, INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT64_SIGNED_MAX]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", [(val,) for val in test_values])

        # And Query "SELECT * FROM <table>" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain integers [0, 42, -2147483648, 2147483647, 9223372036854775807]
        result = sorted([row[0] for row in rows])
        expected = sorted(test_values)
        assert result == expected
        assert_type(result, int)
