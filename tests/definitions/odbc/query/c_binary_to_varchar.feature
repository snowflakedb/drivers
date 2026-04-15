@odbc
Feature: ODBC SQLBindParameter C binary type to VARCHAR conversion
  # Tests for binding SQL_C_BINARY to SQL_VARCHAR (hex encoding).

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string
