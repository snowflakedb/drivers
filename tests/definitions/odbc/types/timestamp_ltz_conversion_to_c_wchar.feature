@odbc
Feature: ODBC TIMESTAMP_LTZ to SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_WCHAR
    Given Snowflake client is logged in with a known session timezone
    When TIMESTAMP_LTZ values are fetched as SQL_C_WCHAR
    Then Wide string representation matches expected format

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_WCHAR buffer too small
    Given Snowflake client is logged in with a known session timezone
    When A TIMESTAMP_LTZ value is fetched into a WCHAR buffer smaller than 20 characters
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_WCHAR truncation
    Given Snowflake client is logged in with a known session timezone
    When A TIMESTAMP_LTZ with fractional seconds is fetched into a WCHAR buffer of 21 characters
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_LTZ value is queried
    Then Indicator returns SQL_NULL_DATA
