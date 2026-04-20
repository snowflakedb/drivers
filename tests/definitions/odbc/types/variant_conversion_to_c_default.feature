@odbc
Feature: ODBC VARIANT to SQL_C_DEFAULT conversions

  @odbc_e2e
  Scenario: VARIANT to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A VARIANT object value is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON object string

  @odbc_e2e
  Scenario: VARIANT to SQL_C_DEFAULT array value
    Given Snowflake client is logged in
    When A VARIANT holding an array is fetched as SQL_C_DEFAULT
    Then The result is a valid JSON array string

  @odbc_e2e
  Scenario: VARIANT to SQL_C_DEFAULT scalar value
    Given Snowflake client is logged in
    When A VARIANT holding a scalar is fetched as SQL_C_DEFAULT
    Then The result is a string representation of the scalar

  @odbc_e2e
  Scenario: VARIANT NULL to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A NULL VARIANT value is queried
    Then Indicator returns SQL_NULL_DATA
