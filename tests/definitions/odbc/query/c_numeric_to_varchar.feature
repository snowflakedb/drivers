@odbc
Feature: ODBC SQLBindParameter C numeric type to VARCHAR conversion
  # Tests for binding SQL_C_NUMERIC (SQL_NUMERIC_STRUCT) to SQL_VARCHAR,
  # including positive/negative values and scale handling.

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind negative SQL_C_NUMERIC with scale to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC with negative scale to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string
