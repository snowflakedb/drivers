@odbc
Feature: ODBC SQLBindParameter C bit type to VARCHAR conversion
  # Tests for binding SQL_C_BIT to SQL_VARCHAR.

  @odbc_e2e
  Scenario: should bind SQL_C_BIT true to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_BIT false to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_BIT value > 1 to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string
