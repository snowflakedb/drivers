@odbc
Feature: ODBC string to type conversions
  # Tests converting Snowflake VARCHAR/STRING type to various ODBC C types
  # This file tests:
  # 1. Successful conversions from string literals representing numbers to numeric ODBC types
  # 2. Failing conversions (invalid strings that cannot be converted to target types)
  # 3. Edge cases like overflow, underflow, and precision loss

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to Integer Types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to signed integer types
    Given Snowflake client is logged in
    When Query selecting string literals representing integers is executed
    Then SQL_C_LONG conversions should work
    And SQL_C_SLONG conversions should work
    And SQL_C_SHORT conversions should work
    And SQL_C_TINYINT conversions should work
    And SQL_C_STINYINT conversions should work
    And SQL_C_SBIGINT conversions should work

  @odbc_e2e
  Scenario: should convert string literals to unsigned integer types
    Given Snowflake client is logged in
    When Query selecting string literals representing unsigned integers is executed
    Then SQL_C_ULONG conversions should work
    And SQL_C_USHORT conversions should work
    And SQL_C_UTINYINT conversions should work
    And SQL_C_UBIGINT conversions should work
    And SQL_C_SSHORT conversions should work

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
  # SUCCESSFUL CONVERSIONS - String to BIT Type
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to SQL_C_BIT
    Given Snowflake client is logged in
    When Query selecting string literals representing boolean values is executed
    Then the string values should be correctly converted to SQL_C_BIT

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to Date/Time Types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to date and time types
    Given Snowflake client is logged in
    When Query selecting string literals representing dates and times is executed
    Then SQL_C_TYPE_DATE conversions should work
    And SQL_C_TYPE_TIME conversions should work
    And SQL_C_TYPE_TIMESTAMP conversions should work

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Strings with leading/trailing whitespace
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals with whitespace to numeric types
    Given Snowflake client is logged in
    When Query selecting string literals with leading/trailing whitespace is executed
    Then the string values should be correctly converted, stripping whitespace

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Decimal strings to integer (truncation)
  # ============================================================================

  @odbc_e2e
  Scenario: should truncate decimal string literals when converting to integer types
    Given Snowflake client is logged in
    When Query selecting string literals with decimal parts is executed
    Then the string values should be truncated when converted to SQL_C_LONG

  # ============================================================================
  # FAILING CONVERSIONS - Non-numeric strings to numeric types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting non-numeric strings to numeric types
    # SQLSTATE 22018 indicates invalid character value for cast
    Given Snowflake client is logged in
    When Query selecting various non-numeric strings is executed
    Then all conversions should fail with SQL_ERROR and SQLSTATE 22018

  @odbc_e2e
  Scenario: should fail converting Unicode string to numeric types
    # SQLSTATE 22018 indicates invalid character value for cast
    Given Snowflake client is logged in
    When Query selecting Unicode string is executed
    And Attempt to get data as SQL_C_LONG
    Then the conversion should fail with SQL_ERROR
    And the SQLSTATE should indicate invalid character value for cast (22018)

  # ============================================================================
  # FAILING CONVERSIONS - Overflow scenarios
  # ============================================================================

  @odbc_e2e
  Scenario: should fail when string value overflows signed integer types
    # SQLSTATE 22003 indicates numeric value out of range
    Given Snowflake client is logged in
    When Query selecting string values that overflow various types is executed
    Then SQL_C_TINYINT should overflow (max 127)
    And SQL_C_SHORT should overflow (max 32767)
    And SQL_C_LONG should overflow (max 2147483647)

  @odbc_e2e
  Scenario: should fail when negative string value used with unsigned types
    # SQLSTATE 22003 indicates numeric value out of range
    Given Snowflake client is logged in
    When Query selecting negative string values is executed
    Then SQL_C_ULONG should fail
    And SQL_C_UTINYINT should fail
    And SQL_C_USHORT should fail

  # ============================================================================
  # FAILING CONVERSIONS - Invalid date/time format strings
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting invalid date/time strings
    # SQLSTATE 22018 indicates invalid character value for cast
    Given Snowflake client is logged in
    When Query selecting invalid date/time strings is executed
    Then SQL_C_TYPE_DATE should fail
    And SQL_C_TYPE_TIME should fail
    And SQL_C_TYPE_TIMESTAMP should fail

  @odbc_e2e
  Scenario: should fail converting alternative date formats to SQL_C_TYPE_DATE
    # Tests various non-standard date formats that should fail conversion
    Given Snowflake client is logged in
    When Query selecting multiple date strings in alternative formats is executed
    And Attempt to get data as SQL_C_TYPE_DATE
    Then the conversion should fail with SQL_ERROR
    And the SQLSTATE should indicate invalid character value for cast (22018)

  @odbc_e2e
  Scenario: should fail converting alternative time formats to SQL_C_TYPE_TIME
    # Tests various non-standard time formats that should fail conversion
    Given Snowflake client is logged in
    When Query selecting multiple time strings in alternative formats is executed
    And Attempt to get data as SQL_C_TYPE_TIME
    Then the conversion should fail with SQL_ERROR
    And the SQLSTATE should indicate invalid character value for cast (22018)

  @odbc_e2e
  Scenario: should fail converting alternative timestamp formats to SQL_C_TYPE_TIMESTAMP
    # Tests various non-standard timestamp formats that should fail conversion
    Given Snowflake client is logged in
    When Query selecting multiple timestamp strings in alternative formats is executed
    And Attempt to get data as SQL_C_TYPE_TIMESTAMP
    Then the conversion should fail with SQL_ERROR
    And the SQLSTATE should indicate invalid character value for cast (22018)

  @odbc_e2e
  Scenario: should convert date-only and time-only strings to SQL_C_TYPE_TIMESTAMP
    # Date-only strings get midnight time, time-only strings get today's date
    Given Snowflake client is logged in
    When Query selecting date-only string is executed
    And Data is retrieved as SQL_C_TYPE_TIMESTAMP
    Then the date components should be correctly parsed
    And the time components should default to midnight
    And the date components should default to today's date
    And the time components should be correctly parsed

  # ============================================================================
  # EDGE CASES - Special floating point strings
  # ============================================================================

  @odbc_e2e
  Scenario: should handle special floating point string conversions
    # Tests inf, -inf, and NaN string conversions
    Given Snowflake client is logged in
    When Query selecting special float strings is executed
    Then inf conversion either succeeds with infinity or fails
    And -inf conversion either succeeds or fails
    And NaN conversion either succeeds with NaN or fails

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL string when converting to numeric and floating point types
    Given Snowflake client is logged in
    When Query selecting NULL is executed
    And Attempt to get data as SQL_C_LONG
    And Attempt to get data as SQL_C_DOUBLE

  # ============================================================================
  # CONVERSION VIA TABLE - String column to numeric types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string column values to numeric types
    Given Snowflake client is logged in
    And A table with VARCHAR column containing numeric strings is created
    When Query selecting from the table is executed
    Then the string values should be correctly converted

  # ============================================================================
  # CONVERSION WITH SQLBindCol
  # ============================================================================

  @odbc_e2e
  Scenario: should convert strings using SQLBindCol
    # Test successful SQL_C_LONG binding
    # Test successful SQL_C_DOUBLE binding
    # Test failed binding for invalid string
    Given Snowflake client is logged in

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
    And scientific notation '1.5e3' should convert correctly (1.5e3 = 1500)
    And very large integer '123456789012345678901234567890' should convert correctly to 18EE90FF6C373E0EE4E3F0AD2
    And NULL should return SQL_NULL_DATA indicator

  # ============================================================================
  # FAILING CONVERSIONS - String to SQL_C_NUMERIC
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting invalid strings to SQL_C_NUMERIC
    Given Snowflake client is logged in
    When Query selecting invalid numeric strings is executed
    Then text should fail with 22018
    And empty string should fail
    And trailing text should fail
    And multiple decimal points should fail

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to SQL_C_BINARY
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to SQL_C_BINARY
    Given Snowflake client is logged in
    When Query selecting various string literals is executed
    Then ASCII string 'hello' should convert to raw bytes
    And empty string should return 0 bytes
    And mixed ASCII with special characters should convert correctly
    And NULL should return SQL_NULL_DATA

  @odbc_e2e
  Scenario: should convert UTF-8 string literals to SQL_C_BINARY
    Given Snowflake client is logged in
    When Query selecting UTF-8 string literals is executed
    Then Japanese '日本語' should convert to UTF-8 bytes (3 chars × 3 bytes each = 9 bytes)
    And Russian 'Привет' should convert to UTF-8 bytes (6 chars × 2 bytes each = 12 bytes)
    And Chinese '你好' should convert to UTF-8 bytes (2 chars × 3 bytes each = 6 bytes)
    And emoji string 'émoji: 😀' should include 4-byte emoji
    And French 'café' should convert correctly (4 chars, 5 bytes due to 'é')
    And Spanish 'Ñoño' should convert correctly
    And musical symbol '𝄞' should convert correctly

  # ============================================================================
  # EDGE CASES - Numeric strings with special formatting
  # ============================================================================

  @odbc_e2e
  Scenario: should handle edge case numeric string formats
    # Tests leading zeros, explicit plus sign, scientific notation
    Given Snowflake client is logged in
    When Query selecting strings with special formatting is executed
    Then leading zeros should be handled correctly
    And explicit plus sign should be handled
    And very small decimal values should convert
    And uppercase E in scientific notation should work

  # ============================================================================
  # FAILING CONVERSIONS - Partial numeric strings
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting partial or malformed numeric strings
    # SQLSTATE 22018 indicates invalid character value for cast
    Given Snowflake client is logged in
    When Query selecting various malformed numeric strings is executed
    Then trailing text should fail for SQL_C_LONG
    And leading text should fail for SQL_C_LONG
    And multiple decimal points should fail for SQL_C_DOUBLE
    And comma as decimal separator should fail for SQL_C_DOUBLE

  # ============================================================================
  # FAILING CONVERSIONS - BIT type edge cases
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting invalid values to SQL_C_BIT
    Given Snowflake client is logged in
    When Query selecting invalid BIT values is executed
    Then non-boolean text should fail with 22018
    And value > 1 should fail with 22003
