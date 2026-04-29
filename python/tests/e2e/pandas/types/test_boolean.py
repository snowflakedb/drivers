"""BOOLEAN type tests for Universal Driver -- pandas consumer.

Arrow bool -> numpy bool (no nulls) or object (with nulls).
"""

from __future__ import annotations

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_bool,
)


# Test constants ported from tests/e2e/types/test_boolean.py
LARGE_RESULT_SET_SIZE = 1_000_000


class TestFetchPandasBooleanTypeCasting:
    """Type-casting coverage for BOOLEAN via fetch_pandas_all."""

    def test_should_cast_boolean_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN" is executed
        df = execute_and_fetch(cursor, "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN")

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_bool, is_bool, is_bool])

        # And Values should match [TRUE, FALSE, TRUE]
        assert get_row(df, 0) == [True, False, True]


class TestFetchPandasBooleanLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    def test_should_select_boolean_literals(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
        df = execute_and_fetch(cursor, "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN")

        # Then Result should contain [TRUE, FALSE]
        assert_dtypes(df, [is_bool, is_bool])
        assert get_row(df, 0) == [True, False]

    def test_should_handle_null_values_from_literals(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN" is executed
        df = execute_and_fetch(cursor, "SELECT FALSE::BOOLEAN, NULL::BOOLEAN, TRUE::BOOLEAN, NULL::BOOLEAN")

        # Then Result should contain [FALSE, NULL, TRUE, NULL]
        assert get_row(df, 0) == [False, None, True, None]

    def test_should_download_large_result_set_with_multiple_chunks_from_generator(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT (id % 2 = 0)::BOOLEAN FROM <generator>" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT (seq8() % 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))",
        )

        # Then Result should contain 500000 TRUE and 500000 FALSE values
        col = get_column(combined, 0)
        assert len(col) == LARGE_RESULT_SET_SIZE
        assert sum(col) == LARGE_RESULT_SET_SIZE // 2


class TestFetchPandasBooleanTable:
    """Table-based scenarios via fetch_pandas_all."""

    def test_should_select_boolean_values_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with columns (BOOLEAN, BOOLEAN, BOOLEAN) exists
        table_name = f"{tmp_schema}.pd_bool_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col1 BOOLEAN, col2 BOOLEAN, col3 BOOLEAN)")

        # And Row (TRUE, FALSE, TRUE) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (TRUE, FALSE, TRUE)")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then Result should contain [TRUE, FALSE, TRUE]
        assert_dtypes(df, [is_bool, is_bool, is_bool])
        assert get_row(df, 0) == [True, False, True]

    def test_should_handle_null_values_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with BOOLEAN column exists
        table_name = f"{tmp_schema}.pd_null_bool"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col BOOLEAN)")

        # And Rows [NULL, TRUE, FALSE] are inserted
        execute_query(f"INSERT INTO {table_name} VALUES (NULL), (TRUE), (FALSE)")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then Result should contain [NULL, TRUE, FALSE] in any order
        assert get_row(df, 0) == [False]
        assert get_row(df, 1) == [True]
        assert get_row(df, 2) == [None]

    def test_should_download_large_result_set_with_multiple_chunks_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with BOOLEAN column exists with 500000 TRUE and 500000 FALSE values
        table_name = f"{tmp_schema}.pd_large_bool"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col BOOLEAN)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT (seq4() % 2 = 0)::BOOLEAN FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT col FROM <table>" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT col FROM {table_name}")

        # Then Result should contain 500000 TRUE and 500000 FALSE values
        col = get_column(combined, 0)
        assert len(col) == LARGE_RESULT_SET_SIZE
        assert sum(col) == LARGE_RESULT_SET_SIZE // 2


@with_paramstyle("qmark")
class TestFetchPandasBooleanBinding:
    """Parameter-binding scenarios via fetch_pandas_all."""

    def test_should_select_boolean_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN" is executed
        # with bound boolean values [TRUE, FALSE, TRUE]
        df = execute_and_fetch(cursor, "SELECT ?::BOOLEAN, ?::BOOLEAN, ?::BOOLEAN", params=(True, False, True))

        # Then Result should contain [TRUE, FALSE, TRUE]
        assert_dtypes(df, [is_bool, is_bool, is_bool])
        assert get_row(df, 0) == [True, False, True]

    def test_should_select_null_boolean_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::BOOLEAN" is executed with bound NULL value
        df = execute_and_fetch(cursor, "SELECT ?::BOOLEAN", params=(None,))

        # Then Result should contain [NULL]
        assert get_row(df, 0) == [None]

    def test_should_insert_boolean_using_parameter_binding(self, execute_query, executemany_insert, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with BOOLEAN column exists
        table_name = f"{tmp_schema}.pd_bool_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col BOOLEAN)")

        # When Boolean values [TRUE, FALSE, NULL] are bulk-inserted using multirow binding
        test_data = [(True,), (False,), (None,)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_data)

        # Then SELECT should return the same values in any order
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")
        assert get_row(df, 0) == [False]
        assert get_row(df, 1) == [True]
        assert get_row(df, 2) == [None]
