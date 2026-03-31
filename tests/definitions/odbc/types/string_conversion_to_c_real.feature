@odbc
Feature: ODBC string to floating point type conversions
  # Tests converting Snowflake VARCHAR/STRING type to floating point ODBC C types:
  # SQL_C_FLOAT, SQL_C_DOUBLE

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to Floating Point Types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to floating point types
    Given Snowflake client is logged in
    When Query selecting string literals representing floating point numbers is executed
    Then SQL_C_FLOAT conversions should work
    And SQL_C_DOUBLE conversions should work
    And integer strings should convert to floating point

  # ============================================================================
  # DATA OUT OF RANGE - String to Floating Point Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting string literals to floating point types when data is out of range
    Given Snowflake client is logged in
    When Query selecting string literals representing floating point numbers is executed
    Then values within range should convert successfully and values exceeding SQL_C_DOUBLE range should fail
    And values exceeding SQL_C_FLOAT range should fail with numeric out of range

  # ============================================================================
  # EDGE CASES - Special floating point string conversions
  # ============================================================================

  @odbc_e2e
  Scenario: should handle special floating point string conversions
    # Tests inf, -inf, and NaN string conversions
    Given Snowflake client is logged in
    When Query selecting special float strings is executed
    Then inf conversion either succeeds with infinity or fails

  # ============================================================================
  # EDGE CASES - Numeric strings with special formatting
  # ============================================================================

  @odbc_e2e
  Scenario: should handle edge case floating point string formats
    # Tests explicit plus sign, scientific notation, very small values
    Given Snowflake client is logged in
    When Query selecting strings with special formatting is executed
    Then explicit plus sign should be handled for floats
    And very small decimal values should convert
    And uppercase E in scientific notation should work

  # ============================================================================
  # FAILING CONVERSIONS - Non-numeric strings to floating point types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting non-numeric strings to floating point types
    # SQLSTATE 22018 indicates invalid character value for cast
    Given Snowflake client is logged in
    When Query selecting various non-numeric strings is executed
    Then text should fail for SQL_C_FLOAT
    And non-numeric text should fail for SQL_C_DOUBLE

  # ============================================================================
  # FAILING CONVERSIONS - Malformed numeric strings
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting malformed numeric strings to floating point types
    # SQLSTATE 22018 indicates invalid character value for cast
    Given Snowflake client is logged in
    When Query selecting various malformed numeric strings is executed
    Then multiple decimal points should fail for SQL_C_DOUBLE
    And comma as decimal separator should fail for SQL_C_DOUBLE

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL string when converting to floating point types
    Given Snowflake client is logged in
    When Query selecting NULL is executed
    Then SQL_C_DOUBLE should return SQL_NULL_DATA indicator

  # ============================================================================
  # CONVERSION WITH SQLBindCol - Floating point types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert strings to floating point types using SQLBindCol
    # Test successful SQL_C_DOUBLE binding
    Given Snowflake client is logged in
    When Query selecting string numeric value is executed with SQLBindCol for SQL_C_DOUBLE
    Then the bound double value should match the string representation

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to SQL_C_NUMERIC
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to SQL_C_NUMERIC
    Given Snowflake client is logged in
    When Query selecting various numeric string formats is executed
    Then positive integer '12345' should convert correctly
    And negative integer '-67890' should convert correctly
    And zero '0' should convert correctly
    And decimal '123.456' should convert correctly with appropriate scale
    And whitespace '  999  ' should be stripped
    And explicit plus sign '+42' should be handled
    And leading zeros '00123' should be handled
    And scientific notation '1.5432e3' should convert correctly (1.5432e3 = 1543)
    And very large integer '123456789012345678901234567890' should convert correctly to 18EE90FF6C373E0EE4E3F0AD2
    And NULL should return SQL_NULL_DATA indicator
