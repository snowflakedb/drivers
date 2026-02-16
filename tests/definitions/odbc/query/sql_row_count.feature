@odbc
Feature: ODBC SQLRowCount function behavior
  # Tests for SQLRowCount based on ODBC specification

  @odbc_e2e
  Scenario: SQLRowCount returns data about number of rows affected.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute the query that returns 1 row
    And SQLRowCount is called to get the number of rows affected
    Then the number of rows affected should be 1

  @odbc_e2e
  Scenario: SQLRowCount returns correct count for INSERT statement.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute an INSERT statement
    And SQLRowCount is called to get the number of rows affected
    Then the number of rows affected should be 3

  @odbc_e2e
  Scenario: SQLRowCount returns correct count for SELECT with many rows.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a query that returns 10 rows
    And SQLRowCount is called to get the number of rows affected
    Then the number of rows affected should be 10
