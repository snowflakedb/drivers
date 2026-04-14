@odbc
Feature: ODBC DATE to SQL_C_CHAR and SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR
    Given Snowflake client is logged in
    When DATE values are fetched as SQL_C_CHAR
    Then String representation matches "yyyy-mm-dd" format for basic, pre-epoch, leap day, epoch, end-of-year, first-of-year, and non-leap dates

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR exact buffer fit
    Given Snowflake client is logged in
    When A DATE value is fetched into an 11-byte buffer
    Then SQL_SUCCESS is returned with indicator 10

  @odbc_e2e
  Scenario: DATE to SQL_C_CHAR truncation
    # BD#41: Old driver returns error instead of 01004 truncation
    Given Snowflake client is logged in
    When A DATE value is fetched into a buffer smaller than 11 bytes
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_CHAR
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: DATE to SQL_C_WCHAR
    Given Snowflake client is logged in
    When DATE values are fetched as SQL_C_WCHAR
    Then Wide string representation matches "yyyy-mm-dd" format

  @odbc_e2e
  Scenario: DATE to SQL_C_WCHAR truncation
    # BD#41: Old driver returns error instead of 01004 truncation
    Given Snowflake client is logged in
    When A DATE value is fetched into a WCHAR buffer smaller than the date string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
