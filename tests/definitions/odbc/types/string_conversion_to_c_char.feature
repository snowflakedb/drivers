@odbc
Feature: ODBC string to character/binary type conversions
  # Tests converting Snowflake VARCHAR/STRING type to character/binary ODBC C types:
  # SQL_C_BINARY, SQL_C_CHAR, SQL_C_WCHAR

  # ============================================================================
  # STRING TRUNCATION TESTS
  # ============================================================================

  @odbc_e2e
  Scenario: should truncate string data when byte length is longer than the buffer length
    Given Snowflake client is logged in
    When Query selecting a long string is executed
    And Attempt to get data with a buffer that is too short
    Then the function should return SQL_SUCCESS_WITH_INFO (truncation occurred)
    And the buffer should contain the truncated string with null terminator
    And the indicator should show the actual length of the original string

  @odbc_e2e
  Scenario: should truncate wide string data when byte length is longer than the buffer length
    Given Snowflake client is logged in
    When Query selecting a long string is executed
    And Attempt to get data with a buffer that is too short
    Then the function should return SQL_SUCCESS_WITH_INFO (truncation occurred)
    And the indicator should show the actual byte length of the original string in wide char format

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

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - UTF-8 String to SQL_C_BINARY
  # ============================================================================

  @odbc_e2e
  Scenario: should convert UTF-8 string literals to SQL_C_BINARY
    # UTF-8 encoded strings are returned as raw bytes
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
  # UTF-16 TO ASCII CONVERSION
  # ============================================================================

  @odbc_e2e
  Scenario: should convert UTF-16 to ASCII with 0x1a substitution when using SQL_C_CHAR
    # ODBC-specific: When reading UTF-16 data using SQL_C_CHAR target type,
    # non-ASCII characters (> 0x7F) should be replaced with 0x1a (SUB character)
    Given Snowflake client is logged in
    When Query selecting strings with non-ASCII Unicode characters is executed
    Then Japanese characters should be replaced with 0x1a (SUB) when reading as SQL_C_CHAR
    And Mixed string should have ASCII preserved and non-ASCII replaced with 0x1a
    And Emojis should all be replaced with 0x1a
    And Greek letters should be replaced with 0x1a
    And Pure ASCII string should remain unchanged
    And Combined string should have ASCII preserved and non-ASCII replaced with 0x1a
