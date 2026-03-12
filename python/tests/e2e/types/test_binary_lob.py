"""BINARY LOB (Large Object) type tests for Universal Driver.

This module tests large BINARY values at the following limits:
  - Default (pre-2025_03 bundle): Maximum 8 MB (8,388,608 bytes)
  - With 2025_03 bundle enabled: Maximum 64 MB (67,108,864 bytes)

Reference: https://docs.snowflake.com/en/release-notes/bcr-bundles/2025_03/bcr-1942
"""

from .utils import assert_connection_is_open


# =============================================================================
# LOB SIZE LIMITS
# =============================================================================
# Default maximum binary size (8 MB)
LOB_8MB_SIZE = 8_388_608

# Extended maximum binary size with 2025_03 bundle (64 MB)
LOB_64MB_SIZE = 67_108_864

# 64 hex characters (32 bytes) to generate binary data on Snowflake side
HEX32 = "000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F"


def assert_binary_equal(result_bin, expected_bin) -> None:
    """Assert two binary values are equal with detailed error reporting.

    Compares in chunks to provide useful error location on mismatch.

    Args:
        result_bin: Actual binary value from query.
        expected_bin: Expected binary value.
    """
    chunk_size = 1024
    len_result = len(result_bin)
    len_expected = len(expected_bin)
    assert len_result == len_expected, f"Length mismatch: {len_result} != {len_expected}"

    for i in range(0, len_expected, chunk_size):
        expected_chunk = expected_bin[i : i + chunk_size]
        result_chunk = result_bin[i : i + chunk_size]
        assert expected_chunk == result_chunk, (
            f"Chunk mismatch near byte {i}: expected {expected_chunk[:32]!r}..., got {result_chunk[:32]!r}..."
        )


class TestBinaryLob:
    """Tests for BINARY LOB (Large Object) handling."""

    def test_should_handle_maximum_default_binary_size(self, execute_query, tmp_schema):
        # Default maximum binary size is 8MB (8,388,608 bytes)
        # This is the limit before enabling the 2025_03 behavior change bundle

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with BINARY column exists
        table_name = f"{tmp_schema}.lob_8mb_table"
        execute_query(f"CREATE TABLE {table_name} (val BINARY)")

        # When Binary value of 8MB size (8,388,608 bytes) is inserted

        # (Note: REPEAT() cannot be used in VALUES clause, must use INSERT ... SELECT)
        N = LOB_8MB_SIZE // 32
        generated_binary = bytes.fromhex(HEX32) * N
        execute_query(f"INSERT INTO {table_name} SELECT TO_BINARY(REPEAT('{HEX32}', {N}), 'HEX')")

        # And Query "SELECT * FROM {table}" is executed
        result_bin, result_len = execute_query(f"SELECT val, LENGTH(val) as len FROM {table_name}", single_row=True)

        # Then the retrieved value size should be 8MB (8,388,608 bytes)
        assert result_len == LOB_8MB_SIZE, f"Expected length {LOB_8MB_SIZE}, got {result_len}"

        # And data integrity should be maintained
        assert isinstance(result_bin, bytearray), f"Expected bytearray, got {type(result_bin).__name__}"
        assert_binary_equal(result_bin, generated_binary)

    def test_should_handle_extended_maximum_binary_size(self, execute_query, tmp_schema):
        # Extended maximum binary size is 64MB (67,108,864 bytes)
        # Requires 2025_03 behavior change bundle

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Table with BINARY(67108864) column exists
        table_name = f"{tmp_schema}.lob_64mb_table"
        execute_query(f"CREATE TABLE {table_name} (val BINARY(67108864))")

        # When Binary value of 64MB size (67,108,864 bytes) is inserted

        # (Note: REPEAT() cannot be used in VALUES clause, must use INSERT ... SELECT)
        N = LOB_64MB_SIZE // 32
        generated_binary = bytes.fromhex(HEX32) * N
        execute_query(f"INSERT INTO {table_name} SELECT TO_BINARY(REPEAT('{HEX32}', {N}), 'HEX')")

        # And Query "SELECT * FROM {table}" is executed
        result_bin, result_len = execute_query(f"SELECT val, LENGTH(val) as len FROM {table_name}", single_row=True)

        # Then the retrieved value size should be 64MB (67,108,864 bytes)
        assert result_len == LOB_64MB_SIZE, f"Expected length {LOB_64MB_SIZE}, got {result_len}"

        # And data integrity should be maintained
        assert isinstance(result_bin, bytearray), f"Expected bytearray, got {type(result_bin).__name__}"
        assert_binary_equal(result_bin, generated_binary)
