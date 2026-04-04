@odbc
Feature: ODBC TIMESTAMP_TZ to SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_WCHAR
    Given Snowflake client is logged in
    When TIMESTAMP_TZ values are fetched as SQL_C_WCHAR
    Then Wide string representation matches UTC time

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_WCHAR buffer too small
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ value is fetched into a WCHAR buffer smaller than 20 characters
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_WCHAR truncation
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ with fractional seconds is fetched into a WCHAR buffer of 21 characters
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: TIMESTAMP_TZ NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_TZ value is queried
    Then Indicator returns SQL_NULL_DATA
