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

  @odbc_e2e
  Scenario: SQLRowCount returns 0 for DDL statements.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a DDL statement
    And SQLRowCount is called to get the number of rows affected
    Then the number of rows affected should be -1

  @odbc_e2e
  Scenario: SQLRowCount returns HY010 when called without executing statement.
    Given Snowflake client is logged in
    When SQLRowCount is called without executing any statement first
    Then SQLRowCount should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)

  @odbc_e2e
  Scenario: SQLRowCount returns correct count for UPDATE statement.
    Given Snowflake client is logged in
    When an UPDATE statement affecting 2 rows is executed
    And SQLRowCount is called
    Then the number of rows affected should be 2

  @odbc_e2e
  Scenario: SQLRowCount returns correct count for DELETE statement.
    Given Snowflake client is logged in
    When a DELETE statement affecting 2 rows is executed
    And SQLRowCount is called
    Then the number of rows affected should be 2

  @odbc_e2e
  Scenario: SQLRowCount returns cached count after SQLFetch has started.
    Given Snowflake client is logged in
    When a SELECT query returning 5 rows is executed
    And some rows are fetched
    Then SQLRowCount should still return the row count

  @odbc_e2e
  Scenario: SQLRowCount updates after re-execution with different INSERT.
    Given Snowflake client is logged in
    When an INSERT of 3 rows is executed
    And a second INSERT of 1 row is executed on the same statement
    Then SQLRowCount should return the updated count
