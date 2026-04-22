@odbc
Feature: ODBC string to SQL_C_NUMERIC conversions

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
