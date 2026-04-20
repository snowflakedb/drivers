@odbc
Feature: ODBC ARRAY to SQL_C_DEFAULT conversions

  @odbc_e2e
  Scenario: ARRAY to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When An ARRAY value is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON array string with correct elements

  @odbc_e2e
  Scenario: ARRAY to SQL_C_DEFAULT empty
    Given Snowflake client is logged in
    When An empty ARRAY is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON empty array string

  @odbc_e2e
  Scenario: ARRAY to SQL_C_DEFAULT nested
    Given Snowflake client is logged in
    When A nested ARRAY is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON nested array string

  @odbc_e2e
  Scenario: ARRAY NULL to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A NULL ARRAY value is queried
    Then Indicator returns SQL_NULL_DATA
