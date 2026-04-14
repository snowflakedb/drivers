@odbc
Feature: ODBC ARRAY to SQL_C_CHAR and SQL_C_WCHAR conversions

  @odbc_e2e
  Scenario: ARRAY to SQL_C_CHAR
    Given Snowflake client is logged in
    When ARRAY values (integer, string, empty, single element, mixed types) are fetched as SQL_C_CHAR
    Then JSON array string representation is returned

  @odbc_e2e
  Scenario: ARRAY to SQL_C_CHAR nested
    Given Snowflake client is logged in
    When Nested and deeply nested ARRAY values are fetched as SQL_C_CHAR
    Then Nested JSON array string is returned

  @odbc_e2e
  Scenario: ARRAY to SQL_C_CHAR large array
    Given Snowflake client is logged in
    When A large ARRAY with 20 elements is fetched as SQL_C_CHAR
    Then All elements are present in the JSON array string

  @odbc_e2e
  Scenario: ARRAY to SQL_C_CHAR truncation
    Given Snowflake client is logged in
    When An ARRAY value is fetched into a buffer smaller than the JSON string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: ARRAY NULL to SQL_C_CHAR
    Given Snowflake client is logged in
    When A NULL ARRAY value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: ARRAY to SQL_C_WCHAR
    Given Snowflake client is logged in
    When ARRAY values are fetched as SQL_C_WCHAR
    Then JSON array wide string representation is returned

  @odbc_e2e
  Scenario: ARRAY to SQL_C_WCHAR truncation
    Given Snowflake client is logged in
    When An ARRAY value is fetched into a WCHAR buffer smaller than the JSON string
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: ARRAY NULL to SQL_C_WCHAR
    Given Snowflake client is logged in
    When A NULL ARRAY value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: ARRAY to SQL_C_CHAR with null elements
    Given Snowflake client is logged in
    When An ARRAY with interleaved null elements is fetched as SQL_C_CHAR
    Then Null elements are represented as JSON null in the array
