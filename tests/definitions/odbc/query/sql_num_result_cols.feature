@odbc
Feature: ODBC SQLNumResultCols function behavior
  # Tests for SQLNumResultCols based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function

  @odbc_e2e
  Scenario: SQLNumResultCols returns 1 for SELECT with single column.
    Given Snowflake client is logged in
    When a SELECT query with one column is executed
    Then SQLNumResultCols should return 1

  @odbc_e2e
  Scenario: SQLNumResultCols returns correct count for SELECT with many columns.
    Given Snowflake client is logged in
    When a SELECT query with 5 columns is executed
    Then SQLNumResultCols should return 5

  @odbc_e2e
  Scenario: SQLNumResultCols returns correct count for SELECT * from table.
    Given Snowflake client is logged in
    When SELECT * is executed on the table
    Then SQLNumResultCols should return 3

  @odbc_e2e
  Scenario: SQLNumResultCols returns correct column count for empty result set.
    Given Snowflake client is logged in
    When a SELECT query with WHERE 1=0 is executed (empty result set but with columns)
    Then SQLNumResultCols should still return the column count (2)

  @odbc_e2e
  Scenario: SQLNumResultCols returns 0 after DDL statement.
    Given Snowflake client is logged in
    When a DDL statement is executed
    Then SQLNumResultCols should return 0 (DDL produces no result set columns)

  @odbc_e2e
  Scenario: SQLNumResultCols returns correct count after calling a stored procedure.
    Given Snowflake client is logged in
    And a stored procedure exists that returns one column
    When the stored procedure is called
    Then SQLNumResultCols should return 1

  @odbc_e2e
  Scenario: SQLNumResultCols returns HY010 when called on freshly allocated statement.
    Given Snowflake client is logged in
    When SQLNumResultCols is called without any prepare or execute
    Then it should return SQL_ERROR with SQLSTATE HY010

  @odbc_e2e
  Scenario: SQLNumResultCols returns column count after SQLPrepare.
    Given Snowflake client is logged in
    When a SELECT statement is prepared but not executed
    Then SQLNumResultCols should return the column count

  @odbc_e2e
  Scenario: SQLNumResultCols updates column count after re-execution with different query.
    Given Snowflake client is logged in
    When a query with 3 columns is executed
    And a different query with 1 column is executed on the same statement
    Then SQLNumResultCols should return the updated column count

  @odbc_e2e
  Scenario: SQLNumResultCols returns same value as SQL_DESC_COUNT of the IRD.
    Given Snowflake client is logged in
    When a query with 4 columns is executed
    And SQLNumResultCols is called
    And the IRD SQL_DESC_COUNT is read
    Then both values should be equal
