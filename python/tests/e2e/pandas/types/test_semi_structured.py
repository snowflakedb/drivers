"""Semi-structured type (VARIANT/OBJECT/ARRAY) tests for Universal Driver -- pandas consumer.

Mirrors every scenario in ``tests/definitions/shared/types/semi_structured.feature``
using ``cursor.fetch_pandas_all()`` / ``cursor.fetch_pandas_batches()``.

Arrow semi-structured -> pandas ``object`` dtype.  Values are JSON strings
(``str`` in Python).  NULL becomes ``None``.
"""

from __future__ import annotations

import json

import pandas as pd

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_string,
)


LARGE_RESULT_SET_SIZE = 20_000


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


class TestFetchPandasSemiStructuredTypeCasting:
    """Type-casting coverage for VARIANT/OBJECT/ARRAY via fetch_pandas_all."""

    def test_should_cast_semi_structured_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
        df = execute_and_fetch(
            cursor,
            "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')",
        )

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_string, is_string, is_string])
        row = get_row(df, 0)
        assert isinstance(row[0], str)
        assert isinstance(row[1], str)
        assert isinstance(row[2], str)


class TestFetchPandasSemiStructuredLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    def test_should_select_semi_structured_literals(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30),
        # OBJECT_CONSTRUCT('a', 1, 'b', 2)" is executed
        df = execute_and_fetch(
            cursor,
            "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)",
        )

        # Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns
        assert_dtypes(df, [is_string, is_string, is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {"key": "value"}
        assert json.loads(row[1]) == [10, 20, 30]
        assert json.loads(row[2]) == {"a": 1, "b": 2}

    def test_should_select_deeply_nested_semi_structured_literals(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
        df = execute_and_fetch(
            cursor,
            'SELECT PARSE_JSON(\'{"a":{"b":[1,2,{"c":true}]}}\')',
        )

        # Then Result should contain the expected nested value
        assert_dtypes(df, [is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {"a": {"b": [1, 2, {"c": True}]}}


class TestFetchPandasSemiStructuredNullLiteral:
    """NULL handling for semi-structured literals via fetch_pandas_all."""

    def test_should_handle_null_semi_structured_values_from_literals(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
        df = execute_and_fetch(cursor, "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY")

        # Then All columns should return null indicators
        assert_dtypes(df, [is_string, is_string, is_string])
        row = get_row(df, 0)
        assert pd.isna(row[0])
        assert pd.isna(row[1])
        assert pd.isna(row[2])


class TestFetchPandasSemiStructuredTable:
    """Table-based scenarios via fetch_pandas_all."""

    def test_should_select_semi_structured_values_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT, OBJECT, and ARRAY columns exists with JSON values
        table_name = f"{tmp_schema}.pd_semi_table"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col_variant VARIANT, col_object OBJECT, col_array ARRAY)"
        )
        execute_query(
            f"INSERT INTO {table_name} SELECT "
            f"PARSE_JSON('{{\"x\":1}}'), "
            f"OBJECT_CONSTRUCT('k', 'v'), "
            f"ARRAY_CONSTRUCT(10, 20)"
        )

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then Data should contain the expected semi-structured values
        assert_dtypes(df, [is_string, is_string, is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {"x": 1}
        assert json.loads(row[1]) == {"k": "v"}
        assert json.loads(row[2]) == [10, 20]

    def test_should_handle_null_semi_structured_values_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT column exists containing NULLs and values
        table_name = f"{tmp_schema}.pd_semi_null"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col VARIANT)")
        execute_query(f"INSERT INTO {table_name} SELECT NULL")
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('{{\"a\":1}}')")
        execute_query(f"INSERT INTO {table_name} SELECT NULL")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then Result should contain [NULL, {"a":1}, NULL]
        assert_dtypes(df, [is_string])
        assert pd.isna(get_row(df, 0)[0])
        assert json.loads(get_row(df, 1)[0]) == {"a": 1}
        assert pd.isna(get_row(df, 2)[0])


class TestFetchPandasSemiStructuredEmptyContainers:
    """Empty JSON container scenarios via fetch_pandas_all."""

    def test_should_handle_empty_json_containers(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
        df = execute_and_fetch(cursor, "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()")

        # Then Each column should return a valid empty container
        assert_dtypes(df, [is_string, is_string, is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {}
        assert json.loads(row[1]) == []
        assert json.loads(row[2]) == {}

    def test_should_handle_empty_json_array_literal(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON('[]')" is executed
        df = execute_and_fetch(cursor, "SELECT PARSE_JSON('[]')")

        # Then Result should be an empty JSON array
        assert_dtypes(df, [is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == []

    def test_should_round_trip_empty_json_containers_through_a_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT, OBJECT, and ARRAY columns exists with empty containers
        table_name = f"{tmp_schema}.pd_semi_empty"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col_variant VARIANT, col_object OBJECT, col_array ARRAY)"
        )
        execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON('{{}}'), OBJECT_CONSTRUCT(), ARRAY_CONSTRUCT()")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then All columns should return valid empty containers
        assert_dtypes(df, [is_string, is_string, is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {}
        assert json.loads(row[1]) == {}
        assert json.loads(row[2]) == []


class TestFetchPandasSemiStructuredUnicode:
    """Unicode content scenarios via fetch_pandas_all."""

    def test_should_handle_json_with_unicode_content(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query returning JSON with unicode characters is executed
        df = execute_and_fetch(cursor, 'SELECT PARSE_JSON(\'{"name":"日本語テスト"}\')')

        # Then Result should preserve the unicode characters
        assert_dtypes(df, [is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {"name": "日本語テスト"}

    def test_should_handle_json_with_unicode_in_keys(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query returning JSON with unicode characters in keys is executed
        df = execute_and_fetch(cursor, 'SELECT PARSE_JSON(\'{"日本語":"テスト"}\')')

        # Then Result should preserve unicode keys and their associated values
        assert_dtypes(df, [is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {"日本語": "テスト"}


class TestFetchPandasSemiStructuredMultipleChunks:
    """Multiple-chunk download scenarios via fetch_pandas_batches."""

    def test_should_download_semi_structured_data_in_multiple_chunks(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj
        # FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) v ORDER BY 1",
        )

        # Then All 20000 rows should be fetched and each should contain a value with "id" key
        assert len(combined) == LARGE_RESULT_SET_SIZE
        col = get_column(combined, 0)
        for val in col:
            parsed = json.loads(val)
            assert "id" in parsed


@with_paramstyle("qmark")
class TestFetchPandasSemiStructuredBinding:
    """Parameter-binding scenarios via fetch_pandas_all."""

    def test_should_select_variant_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string '{"bound":true}'
        df = execute_and_fetch(cursor, "SELECT PARSE_JSON(?)", params=('{"bound":true}',))

        # Then Result should contain a value with "bound" key
        assert_dtypes(df, [is_string])
        row = get_row(df, 0)
        assert json.loads(row[0]) == {"bound": True}

    def test_should_select_null_variant_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
        df = execute_and_fetch(cursor, "SELECT PARSE_JSON(?)", params=(None,))

        # Then Result should be NULL
        assert_dtypes(df, [is_string])
        assert pd.isna(get_row(df, 0)[0])

    def test_should_insert_variant_using_parameter_binding(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with VARIANT column exists
        table_name = f"{tmp_schema}.pd_semi_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col VARIANT)")

        # When JSON values are inserted using parameter binding via PARSE_JSON(?)
        test_values = ['{"a":1}', '{"b":[2,3]}', '{"c":{"d":true}}']
        for val in test_values:
            execute_query(f"INSERT INTO {table_name} SELECT PARSE_JSON(?)", (val,))

        # Then SELECT should return the inserted JSON values
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")
        assert_dtypes(df, [is_string])
        col = get_column(df, 0)
        parsed = [json.loads(v) for v in col]
        assert {"a": 1} in parsed
        assert {"b": [2, 3]} in parsed
        assert {"c": {"d": True}} in parsed
