@odbc
Feature: ODBC string to character type conversions
  # Tests converting Snowflake VARCHAR/STRING type to character ODBC C types:
  # SQL_C_CHAR, SQL_C_WCHAR

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

  # ============================================================================
  # BASIC STRING QUERY AND PARAMETER BINDING
  # ============================================================================

  @odbc_e2e
  Scenario: Test string basic query
    Given A Snowflake connection
    When A string value is inserted and selected via SQL_C_CHAR
    Then The retrieved string matches the inserted value

  @odbc_e2e
  Scenario: Test basic string binding
    Given A Snowflake connection
    When A string value is inserted via parameter binding and selected
    Then The retrieved string matches the bound parameter value
