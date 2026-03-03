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
  Scenario: SQLRowCount returns -1 for ALTER TABLE DDL statement.
    Given Snowflake client is logged in
    When an ALTER TABLE DDL statement is executed
    And SQLRowCount is called
    Then the number of rows affected should be -1

  @odbc_e2e
  Scenario: SQLRowCount returns -1 for DROP TABLE DDL statement.
    Given Snowflake client is logged in
    When a DROP TABLE DDL statement is executed
    And SQLRowCount is called
    Then the number of rows affected should be -1

  @odbc_e2e
  Scenario: SQLRowCount returns correct count for MERGE statement.
    Given Snowflake client is logged in
    When a MERGE statement affecting rows is executed
    And SQLRowCount is called
    Then the number of rows affected should be 2 (1 updated + 1 inserted)

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
  Scenario: SQLRowCount returns total count for DELETE all rows.
    Given Snowflake client is logged in
    When a DELETE without WHERE clause is executed on a table with 4 rows
    And SQLRowCount is called
    Then the number of rows affected should be 4

  @odbc_e2e
  Scenario: SQLRowCount returns total count for UPDATE all rows.
    Given Snowflake client is logged in
    When an UPDATE without WHERE clause is executed on a table with 3 rows
    And SQLRowCount is called
    Then the number of rows affected should be 3

  @odbc_e2e
  Scenario: SQLRowCount returns correct count for INSERT INTO SELECT.
    Given Snowflake client is logged in
    When INSERT INTO ... SELECT copies 3 rows from a source table
    And SQLRowCount is called
    Then the number of rows affected should be 3

  @odbc_e2e
  Scenario: SQLRowCount returns HY010 after SQLFreeStmt SQL_CLOSE.
    Given Snowflake client is logged in
    When a query is executed and then SQLFreeStmt(SQL_CLOSE) resets the statement
    And SQLRowCount is called
    Then SQLRowCount should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)

  @odbc_e2e
  Scenario: SQLRowCount returns HY010 after SQLPrepare without execute.
    Given Snowflake client is logged in
    When a statement is prepared but not executed
    And SQLRowCount is called
    Then SQLRowCount should return SQL_ERROR with SQLSTATE HY010 (Function sequence error)

  @odbc_e2e
  Scenario: SQLRowCount works with SQLPrepare and SQLExecute flow.
    Given Snowflake client is logged in
    When a statement is prepared and executed via SQLPrepare and SQLExecute
    And SQLRowCount is called
    Then the number of rows affected should be 3

  @odbc_e2e
  Scenario: SQLRowCount updates correctly across different DML types on same statement.
    Given Snowflake client is logged in
    When INSERT, UPDATE, and DELETE are executed sequentially on the same statement
    Then SQLRowCount should reflect the count from each operation

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
