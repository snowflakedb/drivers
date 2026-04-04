@odbc
Feature: ODBC TIMESTAMP_NTZ to SQL_C_CHAR conversions

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_CHAR
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ values are fetched as SQL_C_CHAR
    Then String representation matches expected format

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_CHAR fractional truncation
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ with fractional seconds is fetched into a 21-byte buffer
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004 and fractional part truncated

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_CHAR buffer too small
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ value is fetched into a buffer smaller than 20 bytes
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ NULL to SQL_C_CHAR
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_NTZ value is queried
    Then Indicator returns SQL_NULL_DATA
