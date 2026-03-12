"""BINARY type tests for Universal Driver.

This module tests BINARY type and its synonym (VARBINARY) across various scenarios including
literals, table operations, corner cases, NULL handling, parameter binding, and large result sets.

All tests are parameterized to run with each type synonym to verify they behave identically.

Snowflake Binary types: BINARY and VARBINARY are synonymous.
Storage format: Internally stored as hexadecimal, returned in HEX or BASE64 based on BINARY_OUTPUT_FORMAT
Maximum length: 8,388,608 bytes (default), up to 67,108,864 bytes with 2025_03 bundle
Reference: https://docs.snowflake.com/en/sql-reference/data-types-text#binary
"""

import pytest

from ...conftest import with_paramstyle
from .utils import assert_connection_is_open, assert_sequential_values, assert_type


# =============================================================================
# TYPE SYNONYMS
# =============================================================================
# https://docs.snowflake.com/en/sql-reference/data-types-text#binary
BINARY_TYPE_SYNONYMS = [
    "BINARY",
    "VARBINARY",
]
binary_type_parametrize = pytest.mark.parametrize("binary_type", BINARY_TYPE_SYNONYMS)

# =============================================================================
# CORNER CASE VALUES
# =============================================================================
# Corner cases for binary testing:
#   - Empty binary: b'' (0 bytes)
#   - Single null byte: b'\x00'
#   - Single max byte: b'\xff'
#   - All zeros: b'\x00\x00\x00\x00\x00' (5 null bytes)
#   - All ones: b'\xff\xff\xff\xff\xff' (5 bytes of 0xFF)
#   - Embedded nulls: b'\x48\x00\x65\x00' (bytes with embedded 0x00)
#   - NULL value

# Python bytes values and their SQL hex literal representations
CORNER_CASE_VALUES = [
    (b"", "X''"),  # Empty binary (0 bytes)
    (b"\x00", "X'00'"),  # Single null byte
    (b"\xff", "X'FF'"),  # Single max byte
    (b"\x00\x00\x00\x00\x00", "X'0000000000'"),  # All zeros (5 null bytes)
    (b"\xff\xff\xff\xff\xff", "X'FFFFFFFFFF'"),  # All ones (5 bytes of 0xFF)
    (b"\x48\x00\x65\x00", "X'48006500'"),  # Embedded nulls
]

# Happy path test values
HAPPY_PATH_VALUES = [
    (b"Hello", "X'48656C6C6F'"),  # "Hello" in hex
    (b"World", "X'576F726C64'"),  # "World" in hex
    (b"\x01\x23\x45\x67\x89\xab\xcd\xef", "X'0123456789ABCDEF'"),  # Byte sequence
]

# =============================================================================
# LARGE RESULT SET SIZE
# =============================================================================
LARGE_RESULT_SET_SIZE = 30_000


class TestBinaryTypeCasting:
    """Tests for BINARY type casting to appropriate type."""

    @binary_type_parametrize
    def test_should_cast_binary_values_to_appropriate_type(self, execute_query, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY,
        # TO_BINARY('V29ybGQ=', 'BASE64')::BINARY" is executed
        sql = f"SELECT TO_BINARY('48656C6C6F', 'HEX')::{binary_type}, TO_BINARY('V29ybGQ=', 'BASE64')::{binary_type}"
        result = execute_query(sql, single_row=True)

        # Then All values should be returned as appropriate binary type
        assert_type(result, bytearray)

        # And the result should contain binary values:
        assert result == (b"Hello", b"World")


class TestBinaryLiteral:
    """Tests for BINARY type using SELECT with literals (no tables)."""

    @binary_type_parametrize
    def test_should_select_binary_literals(self, execute_query, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Queries selecting binary literals are executed:
        sql = (
            f"SELECT X'48656C6C6F' AS bin1, "
            f"TO_BINARY('48656C6C6F', 'HEX')::{binary_type} as bin2, "
            f"TO_BINARY('ASNFZ4mrze8=', 'BASE64')::{binary_type} as bin3"
        )
        result = execute_query(sql, single_row=True)

        # Then the results should contain expected binary values:
        assert_type(result, bytearray)
        assert result == (bytearray(b"Hello"), bytearray(b"Hello"), bytearray(b"\x01\x23\x45\x67\x89\xab\xcd\xef"))

    @binary_type_parametrize
    def test_should_handle_binary_corner_case_values_from_literals(self, execute_query, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        for expected_val, sql_val in CORNER_CASE_VALUES:
            # When Query selecting corner case binary literals is executed
            result = execute_query(f"SELECT {sql_val}::{binary_type}", single_row=True)

            # Then the result should contain expected corner case binary values
            assert result == (expected_val,), f"Expected {expected_val!r}, got {result[0]!r}"

    @binary_type_parametrize
    def test_should_handle_null_binary_values_from_literals(self, execute_query, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT NULL::{type}, X'ABCD', NULL::{type}" is executed
        sql = f"SELECT NULL::{binary_type}, X'ABCD', NULL::{binary_type}"
        result = execute_query(sql, single_row=True)

        # Then Result should contain [NULL, 0xABCD, NULL]
        assert result == (None, b"\xab\xcd", None)


class TestBinaryTable:
    """Tests for BINARY type using table operations."""

    @binary_type_parametrize
    def test_should_select_binary_values_from_table(self, execute_query, tmp_schema, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with BINARY column is created
        table_name = f"{tmp_schema}.binary_table_test"
        execute_query(f"CREATE TABLE {table_name} (col {binary_type})")

        # And The table is populated with binary values [X'48656C6C6F', X'576F726C64', X'0123456789ABCDEF']
        for _, sql_val in HAPPY_PATH_VALUES:
            execute_query(f"INSERT INTO {table_name} VALUES ({sql_val})")

        # When Query "SELECT * FROM {table} ORDER BY col" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col")

        # Then the result should contain binary values in order:
        result = [row[0] for row in rows]
        expected = sorted([val for val, _ in HAPPY_PATH_VALUES])
        assert_type(result, bytearray)
        assert result == expected

    @binary_type_parametrize
    def test_should_select_corner_case_binary_values_from_table(self, execute_query, tmp_schema, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with BINARY column is created
        table_name = f"{tmp_schema}.binary_corner_case_table_test"
        execute_query(f"CREATE TABLE {table_name} (col {binary_type})")

        # And The table is populated with corner case binary values
        for _, sql_val in CORNER_CASE_VALUES:
            execute_query(f"INSERT INTO {table_name} VALUES ({sql_val})")

        # When Query "SELECT * FROM {table} ORDER BY 1" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY 1")

        # Then the result should contain the inserted corner case binary values
        result = [row[0] for row in rows]
        expected = sorted([expected_val for expected_val, _ in CORNER_CASE_VALUES])
        assert len(result) == len(expected)
        assert_type(result, bytearray)
        assert result == expected

    @binary_type_parametrize
    def test_should_select_null_binary_values_from_table(self, execute_query, tmp_schema, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with BINARY column is created
        table_name = f"{tmp_schema}.binary_null_table_test"
        execute_query(f"CREATE TABLE {table_name} (col {binary_type})")

        # And The table is populated with NULL and non-NULL binary values [NULL, X'ABCD', NULL]
        execute_query(f"INSERT INTO {table_name} VALUES (NULL)")
        execute_query(f"INSERT INTO {table_name} VALUES (X'ABCD')")
        execute_query(f"INSERT INTO {table_name} VALUES (NULL)")

        # When Query "SELECT * FROM {table}" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then there are 3 rows returned
        result = [row[0] for row in rows]
        assert len(result) == 3
        assert_type(result, bytearray, can_be_none=True)
        # And 2 rows should contain NULL values
        assert result.count(None) == 2
        # And 1 row should contain 0xABCD
        assert b"\xab\xcd" in result

    @binary_type_parametrize
    def test_should_select_binary_with_specified_length_from_table(self, execute_query, tmp_schema, binary_type):
        # Tests BINARY(n) with specific length constraints

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with columns (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY) exists
        table_name = f"{tmp_schema}.binary_length_test"
        execute_query(
            f"CREATE TABLE {table_name} (bin5 {binary_type}(5), bin10 {binary_type}(10), bin_default {binary_type})"
        )

        # And Row (X'0102030405', X'01020304050607080910', X'48656C6C6F') is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (X'0102030405', X'01020304050607080910', X'48656C6C6F')")

        # When Query "SELECT * FROM {table}" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain binary values with correct lengths
        assert len(rows) == 1
        bin5, bin10, bin_default = rows[0]
        assert bin5 == b"\x01\x02\x03\x04\x05"
        assert bin10 == b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x10"
        assert bin_default == b"Hello"


@with_paramstyle("qmark")
class TestBinaryBinding:
    """Tests for BINARY type using parameter binding."""

    @binary_type_parametrize
    def test_should_select_binary_literals_using_parameter_binding(self, execute_query, binary_type):
        # SELECT binding test: Uses SELECT ?::BINARY to bind binary values

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT ?::BINARY, ?::BINARY, ?::BINARY" is executed with bound binary values
        # [0x48656C6C6F, 0x576F726C64, 0x0123456789ABCDEF]
        result = execute_query(
            f"SELECT ?::{binary_type}, ?::{binary_type}, ?::{binary_type}",
            (b"Hello", b"World", b"\x01\x23\x45\x67\x89\xab\xcd\xef"),
            single_row=True,
        )

        # Then the result should contain:
        assert_type(result, bytearray)
        assert result == (b"Hello", b"World", b"\x01\x23\x45\x67\x89\xab\xcd\xef")

    @binary_type_parametrize
    def test_should_insert_binary_using_parameter_binding(
        self, execute_query, executemany_insert, tmp_schema, binary_type
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with BINARY column exists
        table_name = f"{tmp_schema}.binary_bind_insert_test"
        execute_query(f"CREATE TABLE {table_name} (col {binary_type})")

        # When Binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x] are inserted using binding
        test_values = [(b"Hello",), (b"World",), (b"\x00",), (b"\xff",), (b"",)]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM {table}" is executed
        rows = execute_query(f"SELECT * FROM {table_name}")

        # Then Result should contain binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x]
        result = [row[0] for row in rows]
        assert len(result) == len(test_values)
        assert_type(result, bytearray)
        expected = set(str(val[0]) for val in test_values)
        returned = set(str(bytes(val)) for val in result)
        assert expected == returned

    @binary_type_parametrize
    def test_should_bind_corner_case_binary_values(self, execute_query, binary_type):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        for corner_case, _ in CORNER_CASE_VALUES:
            # When Query "SELECT ?::BINARY" is executed with each corner case binary value bound
            result = execute_query(f"SELECT ?::{binary_type}", (corner_case,), single_row=True)

            # Then the result should match the bound corner case value
            assert result == (corner_case,), f"Expected {corner_case!r}, got {result[0]!r}"


class TestBinaryMultipleChunks:
    """Tests for BINARY type with multiple chunks downloading."""

    def test_should_download_binary_data_in_multiple_chunks_using_generator(self, execute_query):
        # ~30000 values ensures data is downloaded in at least two chunks

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8')
        # AS bin_val FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v ORDER BY id" is executed

        # Note: We use ROW_NUMBER() instead of seq8() directly to ensure sequential values
        sql = (
            f"SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id, "
            f"TO_BINARY(LPAD(TO_VARCHAR(ROW_NUMBER() OVER (ORDER BY seq8()) - 1), 10, '0'), 'UTF-8') AS bin_val "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY id"
        )
        rows = execute_query(sql)

        # Then there are 30000 rows returned
        assert len(rows) == LARGE_RESULT_SET_SIZE

        # And all returned binary values should match the generated values in order
        assert_sequential_values(rows, LARGE_RESULT_SET_SIZE, transform=lambda i: (i, str(i).zfill(10).encode("utf-8")))

    def test_should_download_binary_data_in_multiple_chunks_from_table(self, execute_query, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with (bin_data BINARY) exists with 30000 sequential binary values
        table_name = f"{tmp_schema}.binary_chunks_table"
        execute_query(f"CREATE TABLE {table_name} (bin_data BINARY)")

        # Insert 30000 sequential binary values using GENERATOR
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM {table} ORDER BY bin_data" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY bin_data")

        # Then there are 30000 rows returned
        assert len(rows) == LARGE_RESULT_SET_SIZE

        # And all returned binary values should match the inserted values in order
        assert_sequential_values(rows, LARGE_RESULT_SET_SIZE, transform=lambda i: (str(i).zfill(10).encode("utf-8"),))
