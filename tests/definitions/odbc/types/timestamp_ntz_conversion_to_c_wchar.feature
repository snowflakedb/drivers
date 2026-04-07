@odbc
Feature: ODBC TIMESTAMP_NTZ to SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_WCHAR
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ values are fetched as SQL_C_WCHAR
    Then Wide string representation matches expected format

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_WCHAR buffer too small
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ value is fetched into a WCHAR buffer smaller than 20 characters
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_WCHAR truncation
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ with fractional seconds is fetched into a WCHAR buffer of 21 characters
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_NTZ value is queried
    Then Indicator returns SQL_NULL_DATA
