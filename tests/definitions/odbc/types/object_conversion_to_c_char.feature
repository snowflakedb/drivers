@odbc
Feature: ODBC OBJECT to SQL_C_CHAR and SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: OBJECT to SQL_C_CHAR
    Given Snowflake client is logged in
    When OBJECT values (simple, multiple keys, empty, mixed types) are fetched as SQL_C_CHAR
    Then JSON object string representation is returned

  @odbc_e2e
  Scenario: OBJECT to SQL_C_CHAR nested
    Given Snowflake client is logged in
    When Nested OBJECT values (nested object, object with array, deeply nested) are fetched as SQL_C_CHAR
    Then Nested JSON object string is returned

  @odbc_e2e
  Scenario: OBJECT to SQL_C_CHAR truncation
    Given Snowflake client is logged in
    When An OBJECT value is fetched into a buffer smaller than the JSON string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: OBJECT NULL to SQL_C_CHAR
    Given Snowflake client is logged in
    When A NULL OBJECT value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: OBJECT to SQL_C_WCHAR
    Given Snowflake client is logged in
    When OBJECT values are fetched as SQL_C_WCHAR
    Then JSON object wide string representation is returned

  @odbc_e2e
  Scenario: OBJECT to SQL_C_WCHAR truncation
    Given Snowflake client is logged in
    When An OBJECT value is fetched into a WCHAR buffer smaller than the JSON string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: OBJECT NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL OBJECT value is queried
    Then Indicator returns SQL_NULL_DATA
