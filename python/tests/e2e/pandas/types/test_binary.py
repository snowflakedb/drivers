"""BINARY type tests for Universal Driver -- pandas consumer.

Arrow binary -> pandas object dtype. Values are bytes/bytearray.
"""

from __future__ import annotations

import pytest

from tests.conftest import with_paramstyle
from tests.e2e.pandas.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_object,
)
from tests.e2e.types.utils import assert_sequential_values


# Test constants ported from tests/e2e/types/test_binary.py
BINARY_TYPE_SYNONYMS = ["BINARY", "VARBINARY"]
binary_type_parametrize = pytest.mark.parametrize("binary_type", BINARY_TYPE_SYNONYMS)

CORNER_CASE_VALUES = [
    (b"", "X''"),
    (b"\x00", "X'00'"),
    (b"\xff", "X'FF'"),
    (b"\x00\x00\x00\x00\x00", "X'0000000000'"),
    (b"\xff\xff\xff\xff\xff", "X'FFFFFFFFFF'"),
    (b"\x48\x00\x65\x00", "X'48006500'"),
]
LARGE_RESULT_SET_SIZE = 30_000


class TestFetchPandasBinaryTypeCasting:
    """Type-casting coverage for the BINARY family via fetch_pandas_all."""

    @binary_type_parametrize
    def test_should_cast_binary_values_to_appropriate_type(self, cursor, binary_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY,
        # TO_BINARY('V29ybGQ=', 'BASE64')::BINARY" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT TO_BINARY('48656C6C6F', 'HEX')::{binary_type}, TO_BINARY('V29ybGQ=', 'BASE64')::{binary_type}",
        )

        # Then All values should be returned as appropriate binary type
        assert_dtypes(df, [is_object, is_object])
        # And the result should contain binary values:
        assert get_row(df, 0) == [b"Hello", b"World"]


class TestFetchPandasBinaryLiteral:
    """SELECT-with-literal coverage via fetch_pandas_all."""

    @binary_type_parametrize
    def test_should_select_binary_literals(self, cursor, binary_type):
        # Given Snowflake client is logged in
        pass

        # When Queries selecting binary literals are executed:
        df = execute_and_fetch(
            cursor,
            f"SELECT X'48656C6C6F'::{binary_type}, X'576F726C64'::{binary_type}, X'0123456789ABCDEF'::{binary_type}",
        )

        # Then the results should contain expected binary values
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [b"Hello", b"World", b"\x01\x23\x45\x67\x89\xab\xcd\xef"]

    @binary_type_parametrize
    def test_should_handle_binary_corner_case_values_from_literals(self, cursor, binary_type):
        # Given Snowflake client is logged in
        pass

        for expected_val, sql_val in CORNER_CASE_VALUES:
            # When Query selecting corner case binary literals is executed
            df = execute_and_fetch(cursor, f"SELECT {sql_val}::{binary_type}")

            # Then the result should contain expected corner case binary values
            assert get_row(df, 0) == [expected_val], f"Expected {expected_val!r}"

    @binary_type_parametrize
    def test_should_handle_null_binary_values_from_literals(self, cursor, binary_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT NULL::{type}, X'ABCD', NULL::{type}" is executed
        df = execute_and_fetch(cursor, f"SELECT NULL::{binary_type}, X'ABCD'::{binary_type}, NULL::{binary_type}")

        # Then Result should contain [NULL, 0xABCD, NULL]
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [None, b"\xab\xcd", None]


class TestFetchPandasBinaryTable:
    """Table-based scenarios via fetch_pandas_all."""

    @binary_type_parametrize
    def test_should_select_binary_values_from_table(self, execute_query, cursor, tmp_schema, binary_type):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with BINARY column is created
        table_name = f"{tmp_schema}.pd_binary_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {binary_type})")

        # And The table is populated with binary values [X'48656C6C6F', X'576F726C64', X'0123456789ABCDEF']
        for sql_val in ["X'48656C6C6F'", "X'576F726C64'"]:
            execute_query(f"INSERT INTO {table_name} VALUES ({sql_val})")

        # When Query "SELECT * FROM {table} ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then the result should contain binary values in order:
        assert_dtypes(df, [is_object])
        assert get_row(df, 0) == [b"Hello"]
        assert get_row(df, 1) == [b"World"]

    @binary_type_parametrize
    def test_should_select_corner_case_binary_values_from_table(self, execute_query, cursor, tmp_schema, binary_type):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with BINARY column is created
        table_name = f"{tmp_schema}.pd_binary_corner_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {binary_type})")

        # And The table is populated with corner case binary values
        for _, sql_val in CORNER_CASE_VALUES:
            execute_query(f"INSERT INTO {table_name} VALUES ({sql_val})")

        # When Query "SELECT * FROM {table} ORDER BY 1" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY 1")

        # Then the result should contain the inserted corner case binary values
        assert_dtypes(df, [is_object])
        assert get_column(df, 0) == sorted(v for v, _ in CORNER_CASE_VALUES)

    @binary_type_parametrize
    def test_should_select_null_binary_values_from_table(self, execute_query, cursor, tmp_schema, binary_type):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with BINARY column is created
        table_name = f"{tmp_schema}.pd_null_binary"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {binary_type})")

        # And The table is populated with NULL and non-NULL binary values [NULL, X'ABCD', NULL]
        execute_query(f"INSERT INTO {table_name} VALUES (NULL), (X'ABCD'), (NULL)")

        # When Query "SELECT * FROM {table}" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then there are 3 rows returned
        assert_dtypes(df, [is_object])
        # And 1 row should contain 0xABCD
        assert get_row(df, 0) == [b"\xab\xcd"]
        # And 2 rows should contain NULL values
        assert get_row(df, 1) == [None]
        assert get_row(df, 2) == [None]

    @binary_type_parametrize
    def test_should_select_binary_with_specified_length_from_table(
        self, execute_query, cursor, tmp_schema, binary_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with columns (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY) exists
        table_name = f"{tmp_schema}.pd_binary_length"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} "
            f"(bin5 {binary_type}(5), bin10 {binary_type}(10), bin_default {binary_type})"
        )

        # And Row (X'0102030405', X'01020304050607080910', X'48656C6C6F') is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (X'0102030405', X'01020304050607080910', X'48656C6C6F')")

        # When Query "SELECT * FROM {table}" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then Result should contain binary values with correct lengths
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [b"\x01\x02\x03\x04\x05", b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10", b"Hello"]

    def test_should_download_binary_data_in_multiple_chunks_using_generator(self, cursor):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS bin_val
        # FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v ORDER BY id" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id, "
            f"TO_BINARY(LPAD(TO_VARCHAR(ROW_NUMBER() OVER (ORDER BY seq8()) - 1), 10, '0'), 'UTF-8') AS bin_val "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY id",
        )

        # Then there are 30000 rows returned
        assert len(combined) == LARGE_RESULT_SET_SIZE
        # And all returned binary values should match the generated values in order
        id_col = get_column(combined, 0)
        bin_col = get_column(combined, 1)
        assert_sequential_values(id_col, LARGE_RESULT_SET_SIZE)
        assert_sequential_values(bin_col, LARGE_RESULT_SET_SIZE, transform=lambda i: str(i).zfill(10).encode("utf-8"))

    def test_should_download_binary_data_in_multiple_chunks_from_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with (bin_data BINARY) exists with 30000 sequential binary values
        table_name = f"{tmp_schema}.pd_binary_chunks_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (bin_data BINARY)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT TO_BINARY(LPAD(TO_VARCHAR(seq4()), 10, '0'), 'UTF-8') "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM {table} ORDER BY bin_data" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY bin_data")

        # Then there are 30000 rows returned
        assert len(combined) == LARGE_RESULT_SET_SIZE

        # And all returned binary values should match the inserted values in order
        col = get_column(combined, 0)
        assert_sequential_values(col, LARGE_RESULT_SET_SIZE, transform=lambda i: str(i).zfill(10).encode("utf-8"))


class TestFetchPandasBinaryVarbinarySynonym:
    """Verify that VARBINARY behaves identically to BINARY."""

    def test_should_handle_varbinary_as_synonym_for_binary(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with VARBINARY column is created
        table_name = f"{tmp_schema}.pd_varbinary_synonym"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col VARBINARY)")

        # And The table is populated with binary values via VARBINARY column
        execute_query(f"INSERT INTO {table_name} VALUES (X'48656C6C6F'), (X'576F726C64')")

        # When Query "SELECT * FROM {table} ORDER BY col" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col")

        # Then the result should match the equivalent BINARY behavior
        assert_dtypes(df, [is_object])
        assert get_row(df, 0) == [b"Hello"]
        assert get_row(df, 1) == [b"World"]


@with_paramstyle("qmark")
class TestFetchPandasBinaryBinding:
    """Parameter-binding scenarios via fetch_pandas_all."""

    @binary_type_parametrize
    def test_should_select_binary_literals_using_parameter_binding(self, cursor, binary_type):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::BINARY, ?::BINARY, ?::BINARY" is executed with bound
        # binary values [0x48656C6C6F, 0x576F726C64, 0x0123456789ABCDEF]
        df = execute_and_fetch(
            cursor,
            f"SELECT ?::{binary_type}, ?::{binary_type}, ?::{binary_type}",
            params=(b"Hello", b"World", b"\x01\x23\x45\x67\x89\xab\xcd\xef"),
        )

        # Then the result should contain:
        assert_dtypes(df, [is_object, is_object, is_object])
        assert get_row(df, 0) == [b"Hello", b"World", b"\x01\x23\x45\x67\x89\xab\xcd\xef"]

    @binary_type_parametrize
    def test_should_insert_binary_using_parameter_binding(
        self, execute_query, executemany_insert, cursor, tmp_schema, binary_type
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with BINARY column exists
        table_name = f"{tmp_schema}.pd_binary_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col {binary_type})")

        # When Binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x] are inserted using binding
        test_data = [(b"Hello",), (b"World",), (b"\x00",), (b"\xff",), (b"",)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_data)

        # And Query "SELECT * FROM {table}" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then Result should contain binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x]
        assert set(get_column(df, 0)) == {b"Hello", b"World", b"\x00", b"\xff", b""}

    @binary_type_parametrize
    def test_should_bind_corner_case_binary_values(self, cursor, binary_type):
        # Given Snowflake client is logged in
        pass

        for corner_case, _ in CORNER_CASE_VALUES:
            # When Query "SELECT ?::BINARY" is executed with each corner case binary value bound
            df = execute_and_fetch(cursor, f"SELECT ?::{binary_type}", params=(corner_case,))

            # Then the result should match the bound corner case value
            assert get_row(df, 0) == [corner_case], f"Expected {corner_case!r}"
