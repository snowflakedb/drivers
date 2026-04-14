@odbc
Feature: ODBC VARIANT to SQL_C_CHAR and SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: VARIANT to SQL_C_CHAR
    Given Snowflake client is logged in
    When VARIANT values (object, string, empty, numeric, boolean) are fetched as SQL_C_CHAR
    Then JSON string representation is returned for each variant type

  @odbc_e2e
  Scenario: VARIANT to SQL_C_CHAR nested
    Given Snowflake client is logged in
    When Deeply nested VARIANT values and arrays of objects are fetched as SQL_C_CHAR
    Then Nested JSON string is returned

  @odbc_e2e
  Scenario: VARIANT to SQL_C_CHAR with special characters
    Given Snowflake client is logged in
    When VARIANT values containing escaped quotes and control characters are fetched
    Then Valid JSON is returned preserving special characters

  @odbc_e2e
  Scenario: VARIANT to SQL_C_CHAR truncation
    Given Snowflake client is logged in
    When A VARIANT value is fetched into a buffer smaller than the JSON string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: VARIANT NULL to SQL_C_CHAR
    Given Snowflake client is logged in
    When A NULL VARIANT value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: VARIANT to SQL_C_WCHAR
    Given Snowflake client is logged in
    When VARIANT values are fetched as SQL_C_WCHAR
    Then JSON wide string representation is returned

  @odbc_e2e
  Scenario: VARIANT to SQL_C_WCHAR truncation
    Given Snowflake client is logged in
    When A VARIANT value is fetched into a WCHAR buffer smaller than the JSON string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: VARIANT NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL VARIANT value is queried
    Then Indicator returns SQL_NULL_DATA
