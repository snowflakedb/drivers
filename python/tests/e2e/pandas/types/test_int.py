"""INT type tests for Universal Driver -- pandas consumer.

INT/INTEGER/BIGINT/SMALLINT/TINYINT/BYTEINT all map to NUMBER(38,0) in Snowflake.
Arrow decimal128(38,0) -> pandas object dtype (precision > 18).
NULL -> None in object columns (no NaN promotion).
"""

from __future__ import annotations

import pytest

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    NULL_FLOAT,
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_object,
)
from tests.e2e.types.utils import assert_sequential_values


# Test constants ported from tests/e2e/types/test_int.py
INT_TYPE_SYNONYMS = ["INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "BYTEINT"]
int_type_parametrize = pytest.mark.parametrize("int_type", INT_TYPE_SYNONYMS)

INT8_SIGNED_MIN = -128
INT8_SIGNED_MAX = 127
INT8_UNSIGNED_MAX = 255

INT16_SIGNED_MIN = -32768
INT16_SIGNED_MAX = 32767
INT16_UNSIGNED_MAX = 65535

INT32_SIGNED_MIN = -2147483648
INT32_SIGNED_MAX = 2147483647
INT32_UNSIGNED_MAX = 4294967295

INT64_MAX = 9223372036854775807
INT64_MIN = -9223372036854775808

INT38_MAX = 99999999999999999999999999999999999999
INT38_MIN = -99999999999999999999999999999999999999

LARGE_RESULT_SET_SIZE = 50_000


class TestFetchPandasIntTypeCasting:
    """Type-casting coverage for the INT family via fetch_pandas_all."""

    @int_type_parametrize
    def test_should_cast_integer_values_to_appropriate_type_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT 0::{int_type}, 1000000::{int_type}, {INT64_MAX}::{int_type}",
        )

        # Then All values should be returned as appropriate type with no precision loss
        assert get_row(df, 0) == [0, 1000000, INT64_MAX]


class TestFetchPandasIntLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    LITERAL_SELECT_TEST_CASES = [
        ("zero", [0], [0]),
        (
            "tinyint",
            [INT8_SIGNED_MIN, INT8_SIGNED_MAX, INT8_UNSIGNED_MAX],
            [INT8_SIGNED_MIN, INT8_SIGNED_MAX, INT8_UNSIGNED_MAX],
        ),
        (
            "smallint",
            [INT16_SIGNED_MIN, INT16_SIGNED_MAX, INT16_UNSIGNED_MAX],
            [INT16_SIGNED_MIN, INT16_SIGNED_MAX, INT16_UNSIGNED_MAX],
        ),
        (
            "int",
            [INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT32_UNSIGNED_MAX],
            [INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT32_UNSIGNED_MAX],
        ),
        ("bigint", [INT64_MIN, INT64_MAX], [INT64_MIN, INT64_MAX]),
    ]

    @int_type_parametrize
    @pytest.mark.parametrize(
        "values,query_values,expected_values",
        LITERAL_SELECT_TEST_CASES,
        ids=[c[0] for c in LITERAL_SELECT_TEST_CASES],
    )
    def test_should_select_integer_values_for_int_and_synonyms(
        self, cursor, int_type, values, query_values, expected_values
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"{v}::{int_type}" for v in query_values)
        df = execute_and_fetch(cursor, f"SELECT {select_cols}")

        # Then Result should contain integers <expected_values>
        assert get_row(df, 0) == expected_values

    @int_type_parametrize
    def test_should_handle_large_integer_values_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT -99999999999999999999999999999999999999::<type>,
        #   99999999999999999999999999999999999999::<type>" is executed
        df = execute_and_fetch(cursor, f"SELECT {INT38_MIN}::{int_type}, {INT38_MAX}::{int_type}")

        # Then Result should contain integers
        # [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        assert_dtypes(df, [is_object, is_object])
        assert get_row(df, 0) == [INT38_MIN, INT38_MAX]

    @int_type_parametrize
    def test_should_handle_null_values_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
        df = execute_and_fetch(cursor, f"SELECT NULL::{int_type}, 42::{int_type}, NULL::{int_type}")

        # Then Result should contain [NULL, 42, NULL]
        assert get_row(df, 0) == pytest.approx([NULL_FLOAT, 42, NULL_FLOAT], nan_ok=True)

    @int_type_parametrize
    def test_should_download_large_result_set_with_multiple_chunks_for_int_and_synonyms(self, cursor, int_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq4()) - 1)::{int_type} as id "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY 1",
        )

        # Then Result should contain 50000 sequentially numbered rows from 0 to 49999
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE)


class TestFetchPandasIntTable:
    """Table-based scenarios via fetch_pandas_all."""

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
                INT64_MAX,
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
                INT64_MAX,
            ],
            False,
        ),
        (
            "negative",
            [-1, INT8_SIGNED_MIN, INT16_SIGNED_MIN, INT32_SIGNED_MIN, INT64_MIN],
            [INT64_MIN, INT32_SIGNED_MIN, INT16_SIGNED_MIN, INT8_SIGNED_MIN, -1],
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
        "values,insert_values,expected_values,has_null",
        TABLE_SELECT_TEST_CASES,
        ids=[c[0] for c in TABLE_SELECT_TEST_CASES],
    )
    def test_should_select_values_from_table_for_int_and_synonyms(
        self, execute_query, cursor, tmp_schema, int_type, values, insert_values, expected_values, has_null
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with values <insert_values>
        table_name = f"{tmp_schema}.pd_int_table_{int_type.lower()}_{values}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {int_type})")
        for v in insert_values:
            execute_query(
                f"INSERT INTO {table_name} VALUES ({v})" if v is not None else f"INSERT INTO {table_name} VALUES (NULL)"
            )

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain integers <expected_values>
        col = get_column(df, 0)
        if has_null:
            assert col == pytest.approx([NULL_FLOAT if v is None else v for v in expected_values], nan_ok=True)
        else:
            assert col == expected_values

    @int_type_parametrize
    def test_should_select_large_integer_values_from_table_for_int_and_synonyms(
        self, execute_query, cursor, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists with values
        # [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        table_name = f"{tmp_schema}.pd_int38_table_{int_type.lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {int_type})")
        execute_query(f"INSERT INTO {table_name} VALUES ({INT38_MAX}), ({INT38_MIN})")

        # When Query "SELECT * FROM <table> ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain integers
        # [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
        assert_dtypes(df, [is_object])
        assert get_column(df, 0) == [INT38_MIN, INT38_MAX]

    def test_should_handle_server_side_arrow_memory_optimization_for_int_columns_on_multiple_chunks(
        self, execute_query, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with four INT columns exists
        table_name = f"{tmp_schema}.pd_different_int_column_sizes"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} "
            f"(col_int8 INT, col_int16 INT, col_int32 INT, col_int64 INT)"
        )

        # And Each column contains values of different magnitudes
        # (50000 rows to span multiple Arrow chunks)
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT 100, 30000, 2000000000, 9000000000000000000 "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table>" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name}")

        # Then Result should contain 50000 rows with all values equal to expected data
        assert len(combined) == LARGE_RESULT_SET_SIZE
        for i in range(len(combined)):
            assert get_row(combined, i) == [100, 30000, 2000000000, 9000000000000000000]


@with_paramstyle("qmark")
class TestFetchPandasIntBinding:
    """Parameter-binding scenarios via fetch_pandas_all."""

    @int_type_parametrize
    def test_should_insert_integer_using_parameter_binding_for_int_and_synonyms(
        self, execute_query, executemany_insert, cursor, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.pd_int_bind_{int_type.lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {int_type})")

        # When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
        test_values = [0, INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT64_MAX]
        for val in test_values:
            cursor.execute(f"INSERT INTO {table_name} VALUES (?)", (val,))

        # And Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain integers [0, -2147483648, 2147483647, 9223372036854775807]
        assert get_column(df, 0) == sorted(test_values)

    @int_type_parametrize
    def test_should_insert_and_select_integers_from_table_using_batch_parameter_binding_for_int_and_synonyms(
        self, execute_query, executemany_insert, cursor, tmp_schema, int_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with <type> column exists
        table_name = f"{tmp_schema}.pd_int_batch_bind_{int_type.lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {int_type})")

        # When Integer values [0, 42, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
        test_values = [0, 42, INT32_SIGNED_MIN, INT32_SIGNED_MAX, INT64_MAX]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", [(val,) for val in test_values])

        # And Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain integers [0, 42, -2147483648, 2147483647, 9223372036854775807]
        assert get_column(df, 0) == sorted(test_values)
