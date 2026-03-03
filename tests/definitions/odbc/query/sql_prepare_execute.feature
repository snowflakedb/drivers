@odbc
Feature: ODBC SQLPrepare / SQLExecute / SQLExecDirect function behavior
  # Tests for SQLPrepare / SQLExecute / SQLExecDirect based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecute-function
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecdirect-function

  # SQLPrepare Basic Functionality

  @odbc_e2e
  Scenario: SQLPrepare + SQLExecute retrieves result from simple SELECT.
    Given Snowflake client is logged in
    When a simple SELECT is prepared and executed
    Then the result should contain the expected value

  @odbc_e2e
  Scenario: SQLPrepare + SQLExecute retrieves result with multiple columns.
    Given Snowflake client is logged in
    When a SELECT with multiple columns is prepared and executed
    Then all columns should be retrievable

  @odbc_e2e
  Scenario: SQLPrepare + SQLExecute can be executed multiple times with SQLCloseCursor between.
    Given Snowflake client is logged in
    When a SELECT is prepared
    And executed a first time
    And the cursor is closed
    And executed a second time
    Then the same result should be returned

  @odbc_e2e
  Scenario: Re-prepare replaces previous statement on same handle.
    Given Snowflake client is logged in
    When a SELECT is prepared and replaced with a different query
    Then the result should come from the second prepared statement

  @odbc_e2e
  Scenario: SQLPrepare with explicit text length (not SQL_NTS).
    Given Snowflake client is logged in
    When a SELECT is prepared with explicit text length
    Then the result should be correct

  @odbc_e2e
  Scenario: SQLPrepare with explicit length shorter than string uses partial SQL.
    Given Snowflake client is logged in
    When a SELECT is prepared with a length shorter than the full string
    Then the result should reflect the truncated query

  @odbc_e2e
  Scenario: SQLNumResultCols available after SQLPrepare without execute.
    Given Snowflake client is logged in
    When a SELECT with 3 columns is prepared
    Then SQLNumResultCols should return the column count without needing execute

  @odbc_e2e
  Scenario: SQLDescribeCol available after SQLPrepare without execute.
    Given Snowflake client is logged in
    When a SELECT is prepared
    Then SQLDescribeCol should return metadata for the prepared column

  # SQLPrepareW (wide variant)

  @odbc_e2e
  Scenario: SQLPrepareW + SQLExecute basic flow.
    Given Snowflake client is logged in
    When a SELECT is prepared using the wide variant
    Then the result should be correct

  @odbc_e2e
  Scenario: SQLPrepareW with Unicode content in query.
    Given Snowflake client is logged in
    When a SELECT with Unicode string literal is prepared using SQLPrepareW
    Then the Unicode content should be correctly returned

  # SQLExecute Behavior

  @odbc_e2e
  Scenario: SQLExecute without prior SQLPrepare returns HY010.
    Given Snowflake client is logged in
    When SQLExecute is called without a prior SQLPrepare
    Then it should return SQL_ERROR with SQLSTATE HY010

  @odbc_e2e
  Scenario: SQLExecute with bound parameters via SQLBindParameter.
    Given Snowflake client is logged in
    When a parameterized query is prepared and parameters are bound
    Then the bound parameter value should be returned

  @odbc_e2e
  Scenario: SQLExecute with different parameter values on re-execution.
    Given Snowflake client is logged in
    When a parameterized query is prepared
    And executed with value 10
    And cursor is closed
    And re-executed with value 20
    Then the new parameter value should be returned

  @odbc_e2e
  Scenario: SQLExecute returns 24000 when cursor is not closed before re-execute of SELECT.
    Given Snowflake client is logged in
    When a SELECT is prepared and executed
    And SQLExecute is called again without closing the cursor
    Then it should return SQL_ERROR with SQLSTATE 24000

  # SQLExecDirect Enhancements

  @odbc_e2e
  Scenario: SQLExecDirectW basic flow.
    Given Snowflake client is logged in
    When a SELECT is executed via SQLExecDirectW
    Then the result should be correct

  @odbc_e2e
  Scenario: SQLExecDirect with bound parameters via SQLBindParameter.
    Given Snowflake client is logged in
    When a parameter is bound before calling SQLExecDirect
    Then the bound parameter value should be returned

  # SQLPrepare Error Cases

  @odbc_e2e
  Scenario: SQLPrepare with null statement handle returns SQL_INVALID_HANDLE.
    When SQLPrepare is called with a null statement handle
    Then it should return SQL_INVALID_HANDLE

  @odbc_e2e
  Scenario: SQLPrepare with null SQL text pointer returns HY009.
    Given Snowflake client is logged in
    When SQLPrepare is called with a null SQL text pointer
    Then it should return SQL_ERROR with SQLSTATE HY009

  @odbc_e2e
  Scenario: SQLPrepare with negative TextLength returns HY090.
    Given Snowflake client is logged in
    When SQLPrepare is called with a negative text length
    Then it should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLPrepare with zero TextLength returns HY090.
    Given Snowflake client is logged in
    When SQLPrepare is called with zero text length
    Then it should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLPrepare with empty SQL string returns HY090.
    Given Snowflake client is logged in
    When SQLPrepare is called with an empty SQL string
    Then it should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLPrepare with invalid SQL syntax returns 42000.
    Given Snowflake client is logged in
    When SQLPrepare is called with invalid SQL syntax
    Then it should return SQL_ERROR with SQLSTATE 42000

  @odbc_e2e
  Scenario: SQLPrepare with cursor already open returns 24000.
    Given Snowflake client is logged in
    And a query has been executed leaving a cursor open
    When SQLPrepare is called while the cursor is still open
    Then it should return SQL_ERROR with SQLSTATE 24000

  # DDL / DML Edge Cases

  @odbc_e2e
  Scenario: DDL via SQLPrepare + SQLExecute.
    Given Snowflake client is logged in
    When a CREATE TABLE is prepared and executed
    Then the table should exist

  @odbc_e2e
  Scenario: DML returning SQL_NO_DATA via SQLPrepare + SQLExecute.
    Given Snowflake client is logged in
    When a DELETE that affects no rows is prepared and executed
    Then it should return SQL_NO_DATA

  @odbc_e2e
  Scenario: INSERT via SQLPrepare + SQLExecute with verify.
    Given Snowflake client is logged in
    When an INSERT is prepared with bound parameters and executed
    Then the inserted row should be retrievable
