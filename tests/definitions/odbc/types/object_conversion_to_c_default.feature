@odbc
Feature: ODBC OBJECT to SQL_C_DEFAULT conversions

  @odbc_e2e
  Scenario: OBJECT to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When An OBJECT value is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON object string

  @odbc_e2e
  Scenario: OBJECT to SQL_C_DEFAULT empty
    Given Snowflake client is logged in
    When An empty OBJECT is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON empty object string

  @odbc_e2e
  Scenario: OBJECT to SQL_C_DEFAULT nested
    Given Snowflake client is logged in
    When A nested OBJECT is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON nested object string

  @odbc_e2e
  Scenario: OBJECT NULL to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A NULL OBJECT value is queried
    Then Indicator returns SQL_NULL_DATA
