"""STRING type tests for Universal Driver -- pandas consumer.

Arrow string -> pandas StringDtype (pandas 3.0+) or object (pandas < 3.0).
Values are Python str. NULL -> NaN (StringDtype) or None (object).
"""

from __future__ import annotations

import pandas as pd
import pytest

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_string_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
)
from tests.e2e.types.utils import assert_sequential_values


# Test constants ported from tests/e2e/types/test_string.py
STRING_TYPE_SYNONYMS = [
    "VARCHAR",
    "CHAR",
    "CHARACTER",
    "NCHAR",
    "STRING",
    "TEXT",
    "VARCHAR2",
    "NVARCHAR",
    "NVARCHAR2",
    "CHAR VARYING",
    "NCHAR VARYING",
]
string_type_parametrize = pytest.mark.parametrize("string_type", STRING_TYPE_SYNONYMS)

CORNER_CASE_VALUES = [
    ("", "''"),
    ("X", "'X'"),
    ("   ", "'   '"),
    ("\t", "'\\t'"),
    ("\n", "'\\n'"),
    ("⛄", "'⛄'"),
    ("日本語テスト", "'日本語テスト'"),
    ("'", "''''"),
    ("\\", "'\\\\'"),
    (None, "NULL"),
    ("y̆es", "'y̆es'"),
    ("𝄞", "'𝄞'"),
]
LARGE_RESULT_SET_SIZE = 10_000


class TestFetchPandasStringTypeCasting:
    """Type-casting coverage for the STRING family via fetch_pandas_all."""

    @string_type_parametrize
    def test_should_cast_string_values_to_appropriate_type_for_string_and_synonyms(self, cursor, string_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 'hello'::<type>, 'Hello World'::<type>,
        # '日本語テスト'::<type>" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT 'hello'::{string_type}(32), 'Hello World'::{string_type}(32), '日本語テスト'::{string_type}(32)",
        )

        # Then All values should be returned as appropriate type
        assert_string_dtypes(df)
        assert get_row(df, 0) == ["hello", "Hello World", "日本語テスト"]


class TestFetchPandasStringLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    @string_type_parametrize
    def test_should_select_hardcoded_string_literals(self, cursor, string_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT 'hello' AS str1, 'Hello World' AS str2, 'Snowflake Driver Test' AS str3" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT 'hello'::{string_type}(32), 'Hello World'::{string_type}(32), "
            f"'Snowflake Driver Test'::{string_type}(32)",
        )

        # Then the result should contain:
        assert_string_dtypes(df)
        assert get_row(df, 0) == ["hello", "Hello World", "Snowflake Driver Test"]

    @string_type_parametrize
    def test_should_select_string_literals_with_corner_case_values(self, cursor, string_type):
        # Given Snowflake client is logged in
        pass

        # When Query selecting corner case string literals is executed
        type_cast = f"::{string_type}(32)"
        # Then the result should contain expected corner case string values
        for expected_val, sql_val in CORNER_CASE_VALUES:
            if expected_val is None:
                continue
            df = execute_and_fetch(cursor, f"SELECT {sql_val}{type_cast}")
            assert get_row(df, 0) == [expected_val]

        # NULL literal
        df = execute_and_fetch(cursor, f"SELECT NULL{type_cast}")
        assert pd.isna(get_row(df, 0)[0])

    @string_type_parametrize
    def test_should_download_string_data_in_multiple_chunks(self, cursor, string_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT seq8() AS id, TO_VARCHAR(seq8()) AS str_val
        # FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id, "
            f"TO_VARCHAR(ROW_NUMBER() OVER (ORDER BY seq8()) - 1)::{string_type}(32) AS str_val "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY id",
        )

        # Then there are 10000 rows returned and all string values should match the generated values in order
        assert len(combined) == LARGE_RESULT_SET_SIZE
        id_col = get_column(combined, 0)
        str_col = get_column(combined, 1)
        assert_sequential_values(id_col, LARGE_RESULT_SET_SIZE)
        assert_sequential_values(str_col, LARGE_RESULT_SET_SIZE, transform=str)


class TestFetchPandasStringTable:
    """Table-based scenarios via fetch_pandas_all."""

    @string_type_parametrize
    def test_should_select_hardcoded_string_values_from_table(self, execute_query, cursor, tmp_schema, string_type):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.pd_string_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (val {string_type}(32))")

        # And The table is populated with string values
        for v in ["hello", "Hello World", "Snowflake Driver Test"]:
            execute_query(f"INSERT INTO {table_name} VALUES ('{v}')")

        # When Query "SELECT * FROM {table}" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY val")

        # Then the result should contain the inserted hardcoded string values
        assert_string_dtypes(df)
        assert get_column(df, 0) == ["Hello World", "Snowflake Driver Test", "hello"]

    @string_type_parametrize
    def test_should_select_corner_case_string_values_from_table(self, execute_query, cursor, tmp_schema, string_type):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.pd_string_corner"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (val {string_type}(32))")

        # And The table is populated with corner case string values
        for _, sql_val in CORNER_CASE_VALUES:
            execute_query(f"INSERT INTO {table_name} VALUES ({sql_val})")

        # When Query "SELECT * FROM {table}" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then the result should contain the inserted corner case string values
        assert_string_dtypes(df)
        col = [None if pd.isna(v) else v for v in get_column(df, 0)]
        assert set(col) == {v for v, _ in CORNER_CASE_VALUES}


@with_paramstyle("qmark")
class TestFetchPandasStringBinding:
    """Parameter-binding scenarios via fetch_pandas_all."""

    @string_type_parametrize
    def test_should_select_string_literals_using_parameter_binding(self, cursor, string_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::VARCHAR, ?::VARCHAR, ?::VARCHAR" is executed with
        # bound string values ['hello', 'Hello World', '日本語テスト']
        df = execute_and_fetch(
            cursor,
            f"SELECT ?::{string_type}(32), ?::{string_type}(32), ?::{string_type}(32)",
            params=("hello", "Hello World", "日本語テスト"),
        )

        # Then the result should contain:
        assert_string_dtypes(df)
        assert get_row(df, 0) == ["hello", "Hello World", "日本語テスト"]

    @string_type_parametrize
    def test_should_insert_and_select_back_hardcoded_string_values_using_parameter_binding(
        self, execute_query, executemany_insert, cursor, tmp_schema, string_type
    ):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.pd_string_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (val {string_type}(64))")

        # When String value 'Test binding value 日本語' is inserted using parameter binding
        test_value = "Test binding value 日本語"
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", [(test_value,)])

        # And Query "SELECT * FROM {table}" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then the result should contain the bound string value 'Test binding value 日本語'
        assert get_column(df, 0) == [test_value]

    @string_type_parametrize
    def test_should_select_corner_case_string_values_using_parameter_binding(self, cursor, string_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::VARCHAR" is executed with each corner case string value bound
        for corner_case, _ in CORNER_CASE_VALUES:
            if corner_case is None:
                continue
            df = execute_and_fetch(cursor, f"SELECT ?::{string_type}(32)", params=(corner_case,))
            # Then the result should match the bound corner case value
            assert get_row(df, 0) == [corner_case]

        # NULL binding
        df = execute_and_fetch(cursor, f"SELECT ?::{string_type}(32)", params=(None,))
        assert pd.isna(get_row(df, 0)[0])
