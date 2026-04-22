@odbc
Feature: ODBC TIME to SQL_C_CHAR and SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: TIME to SQL_C_CHAR
    Given Snowflake client is logged in
    When A basic TIME is fetched as SQL_C_CHAR
    Then String representation matches expected format
    When A TIME with fractional seconds is fetched as SQL_C_CHAR
    Then String includes fractional seconds
    When Midnight TIME is fetched as SQL_C_CHAR
    Then String representation is all zeros
    When End-of-day TIME is fetched as SQL_C_CHAR
    Then String representation matches

  @odbc_e2e
  Scenario: TIME to SQL_C_CHAR exact buffer fit
    Given Snowflake client is logged in
    When A TIME value is fetched into a 9-byte buffer
    Then SQL_SUCCESS is returned with indicator 8

  @odbc_e2e
  Scenario: TIME to SQL_C_CHAR chunked retrieval
    Given Snowflake client is logged in
    When A TIME with fractional seconds is fetched via two sequential SQLGetData calls with a 10-byte buffer
    Then The first call returns partial data with 01004 and the second call returns the remainder

  @odbc_e2e
  Scenario: TIME to SQL_C_CHAR fractional truncation
    Given Snowflake client is logged in
    When A TIME with fractional seconds is fetched into a 9-byte buffer
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004 and fractional part truncated

  @odbc_e2e
  Scenario: TIME to SQL_C_CHAR buffer too small
    Given Snowflake client is logged in
    When A TIME value is fetched into a buffer smaller than the time string
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIME NULL to SQL_C_CHAR
    Given Snowflake client is logged in
    When A NULL TIME value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: TIME to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A basic TIME is fetched as SQL_C_WCHAR
    Then Wide string representation matches expected format
    When A TIME with fractional seconds is fetched as SQL_C_WCHAR
    Then Wide string includes fractional seconds
    When Midnight TIME is fetched as SQL_C_WCHAR
    Then Wide string representation is all zeros

  @odbc_e2e
  Scenario: TIME to SQL_C_WCHAR exact buffer fit
    Given Snowflake client is logged in
    When A TIME value is fetched into a WCHAR buffer of exactly 9 characters
    Then SQL_SUCCESS is returned with the correct wide string

  @odbc_e2e
  Scenario: TIME to SQL_C_WCHAR fractional truncation
    Given Snowflake client is logged in
    When A TIME with fractional seconds is fetched into a WCHAR buffer of 9 characters
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: TIME to SQL_C_WCHAR buffer too small
    Given Snowflake client is logged in
    When A TIME value is fetched into a WCHAR buffer smaller than the time string
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIME NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL TIME value is queried
    Then Indicator returns SQL_NULL_DATA
