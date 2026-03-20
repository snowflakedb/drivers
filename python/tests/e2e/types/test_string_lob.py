"""STRING LOB (Large Object) type tests for Universal Driver.

This module tests large VARCHAR values at the following limits:
  - Historical limit: 16 MB (16,777,216 bytes) per value
  - Increased LOB Size feature: up to 128 MB (134,217,728 bytes) per value

Reference: https://docs.snowflake.com/en/sql-reference/data-types-text
"""

# =============================================================================
# LOB SIZE LIMITS
# =============================================================================
# Historical LOB limit (16 MB)
LOB_16MB_SIZE = 16_777_216

# Maximum LOB limit with Increased LOB Size feature (128 MB)
LOB_128MB_SIZE = 134_217_728

# 64 characters to generate strings on Snowflake side
CHARS64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789()"


def assert_strings_equal(result_str, generated_string):
    chunk_size = 1024
    len_result_str = len(result_str)
    len_generated_string = len(generated_string)
    assert len_result_str == len_generated_string, f"Length mismatch: {len_result_str} != {len_generated_string}"
    for i in range(0, len(generated_string), chunk_size):
        generated_chunk = generated_string[i : i + chunk_size]
        result_chunk = result_str[i : i + chunk_size]
        assert generated_chunk == result_chunk, (
            f"Chunk mismatch near index {i * chunk_size}: expected '{generated_chunk}', got '{result_chunk}'"
        )


class TestStringLob:
    """Tests for STRING LOB (Large Object) handling."""

    def test_should_handle_lob_string_at_historical_16_mb_limit(self, execute_query, tmp_schema):
        # Corner case: string at the historical LOB limit (16 MB = 16,777,216 bytes)

        # Given Snowflake client is logged in
        pass

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.lob_16mb_table"
        execute_query(f"CREATE TABLE {table_name} (val VARCHAR)")

        # When A string of 16777216 ASCII characters is generated and inserted

        # (Note: REPEAT() cannot be used in VALUES clause, must use INSERT ... SELECT)
        N = LOB_16MB_SIZE // len(CHARS64)
        generated_string = CHARS64 * N
        execute_query(f"INSERT INTO {table_name} SELECT REPEAT('{CHARS64}', {N})")

        # And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
        result_str, result_len = execute_query(f"SELECT val, LENGTH(val) as len FROM {table_name}", single_row=True)

        # Then the result should show length 16777216
        assert result_len == LOB_16MB_SIZE, f"Expected length {LOB_16MB_SIZE}, got {result_len}"

        # And the returned string should exactly match the generated string
        assert isinstance(result_str, str), f"Expected str, got {type(result_str).__name__}"
        assert_strings_equal(result_str, generated_string)

    def test_should_handle_lob_string_at_maximum_128_mb_limit_with_increased_lob_size(self, execute_query, tmp_schema):
        # Corner case: string at maximum LOB limit (128 MB) - requires Increased LOB Size feature

        # Given Snowflake client is logged in
        pass

        # And A temporary table with VARCHAR column is created
        table_name = f"{tmp_schema}.lob_128mb_table"
        execute_query(f"CREATE TABLE {table_name} (val VARCHAR(134217728))")

        # When A string of 134217728 ASCII characters is generated and inserted

        # (Note: REPEAT() cannot be used in VALUES clause, must use INSERT ... SELECT)
        N = LOB_128MB_SIZE // len(CHARS64)
        generated_string = CHARS64 * N
        execute_query(f"INSERT INTO {table_name} SELECT REPEAT('{CHARS64}', {N})")

        # And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
        result_str, result_len = execute_query(f"SELECT val, LENGTH(val) as len FROM {table_name}", single_row=True)

        # Then the result should show length 134217728
        assert result_len == LOB_128MB_SIZE, f"Expected length {LOB_128MB_SIZE}, got {result_len}"

        # And the returned string should exactly match the generated string
        assert isinstance(result_str, str), f"Expected str, got {type(result_str).__name__}"
        assert_strings_equal(result_str, generated_string)
