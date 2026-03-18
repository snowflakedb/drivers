@odbc
Feature: ODBC float integer-valued (.0) conversions to fixed-width C types
  # Tests that FLOAT values with no fractional part (.0) convert correctly
  # to integer C types without triggering 01S07 fractional-truncation warning,
  # and that boundary values at i32/u32/i64 limits are handled correctly.

  # ============================================================================
  # Small .0 values to integer C types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert small integer-valued floats to all integer C types
    # 0.0, 1.0, -1.0 as FLOAT should convert without fractional truncation
    # to signed integer types. -1.0 to unsigned types should return 22003.
    Given Snowflake client is logged in
    When Float values 0.0, 1.0, and -1.0 are queried for type conversion
    Then 0.0 should convert to all integer C types without truncation
    And 1.0 should convert to all integer C types without truncation
    And -1.0 should convert to signed integer C types without truncation
    And -1.0 should return 22003 for unsigned integer C types

  # ============================================================================
  # i32/u32 boundary .0 values
  # ============================================================================

  @odbc_e2e
  Scenario: should handle i32 and u32 boundary values stored as float
    # Tests exact boundary values for 32-bit integer types (all exactly
    # representable in f64). Verifies success at exact boundaries and
    # 22003 overflow one step beyond.
    Given Snowflake client is logged in
    When Boundary float values at i32 and u32 limits are queried
    Then i32 max 2147483647.0 should succeed for SQL_C_LONG and wider types
    And i32 min -2147483648.0 should succeed for SQL_C_LONG and wider signed types
    And u32 max 4294967295.0 should succeed for SQL_C_ULONG and wider types
    And 2147483648.0 should succeed for SQL_C_ULONG and SQL_C_SBIGINT but overflow SQL_C_LONG
    And 4294967296.0 should succeed for SQL_C_SBIGINT but overflow SQL_C_LONG and SQL_C_ULONG

  # ============================================================================
  # Large .0 values to wider C types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert large integer-valued floats to wider types and strings
    # 2^53 is the largest integer exactly representable in f64.
    # Tests conversion to SQL_C_SBIGINT, SQL_C_UBIGINT, SQL_C_DOUBLE, and SQL_C_CHAR.
    Given Snowflake client is logged in
    When Large integer-valued float values are queried
    Then 2^53 should convert exactly to SQL_C_SBIGINT and SQL_C_UBIGINT
    And Large integer-valued floats should convert exactly to SQL_C_DOUBLE
    And Large integer-valued floats should render correctly as SQL_C_CHAR strings

  # ============================================================================
  # .0 values to SQL_C_FLOAT (f64 -> f32)
  # ============================================================================

  @odbc_e2e
  Scenario: should convert integer-valued floats to SQL_C_FLOAT
    # Integer-valued doubles that are exactly representable in f32
    # (small values and powers of two within f32 mantissa range).
    Given Snowflake client is logged in
    When Integer-valued float values are queried for f32 conversion
    Then Small integer-valued floats should convert exactly to SQL_C_FLOAT
    And Power-of-two floats within f32 range should convert exactly to SQL_C_FLOAT

  # ============================================================================
  # .0 values to SQL_C_NUMERIC — boundary values
  # ============================================================================

  @odbc_e2e
  Scenario: should encode large integer-valued floats correctly in SQL_C_NUMERIC
    # Verifies SQL_NUMERIC_STRUCT encoding (sign + val[] bytes) for
    # big integer-valued floats at i32, u32, and 2^53 boundaries.
    Given Snowflake client is logged in
    When Large integer-valued float values are queried for SQL_C_NUMERIC conversion
    Then i32 max should encode correctly in SQL_NUMERIC_STRUCT
    And i32 min should encode as negative in SQL_NUMERIC_STRUCT
    And u32 max should encode correctly in SQL_NUMERIC_STRUCT
    And 2^32 and 2^53 should encode correctly in SQL_NUMERIC_STRUCT
