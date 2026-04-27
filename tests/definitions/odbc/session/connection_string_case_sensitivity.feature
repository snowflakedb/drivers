@odbc
Feature: Connection string key case insensitivity

  Connection string keys (SERVER, UID, PWD, ACCOUNT, etc.) should be
  matched case-insensitively so that users can write them in any
  combination of upper, lower, or mixed case.

  @odbc_e2e
  Scenario: connection string keys are case-insensitive (lowercase)
    Given Snowflake ODBC connection string uses all-lowercase key names
    When Connection is established and "SELECT 1" is executed
    Then the query should succeed and return 1

  @odbc_e2e
  Scenario: connection string keys are case-insensitive (mixed case)
    Given Snowflake ODBC connection string uses mixed-case key names
    When Connection is established and "SELECT 1" is executed
    Then the query should succeed and return 1
