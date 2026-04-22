@odbc
Feature: ODBC VARIANT to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: VARIANT to SQL_C_BINARY
    Given Snowflake client is logged in
    When A VARIANT object value is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as object

  @odbc_e2e
  Scenario: VARIANT to SQL_C_BINARY array value
    Given Snowflake client is logged in
    When A VARIANT holding an array is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as array

  @odbc_e2e
  Scenario: VARIANT to SQL_C_BINARY empty object
    Given Snowflake client is logged in
    When A VARIANT holding an empty object is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as empty object

  @odbc_e2e
  Scenario: VARIANT to SQL_C_BINARY nested value
    Given Snowflake client is logged in
    When A VARIANT holding nested JSON is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable with nested structure

  @odbc_e2e
  Scenario: VARIANT NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL VARIANT value is queried
    Then Indicator returns SQL_NULL_DATA
