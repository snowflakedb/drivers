@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: Multistatement cursor shape (SQLNumResultCols + SQLRowCount)

  Asserts SQL_C API return codes for every result set produced by a
  multi-statement execution. ODBC-only: covers SQLNumResultCols /
  SQLRowCount / SQLFetch behaviour per child, which has no equivalent in
  the other driver surfaces.

  @odbc_e2e
  Scenario: should report correct cursor shape for each result set in a DDL + DML + DDL batch
    Given Snowflake client is logged in
    When Multistatement query with CREATE TABLE, INSERT, and DROP is executed
    Then the CREATE TABLE result set reports no cursor and unknown row count
    And fetching on the CREATE TABLE result set does not return a row
    And the INSERT result set reports no cursor and a row count matching the inserted rows
    And the DROP TABLE result set reports no cursor and unknown row count

  @odbc_e2e
  Scenario: should not open a cursor for any statement in a TCL-only batch
    Given Snowflake client is logged in
    When Multistatement query with BEGIN, ALTER SESSION, and COMMIT is executed
    Then every result set reports no cursor and unknown row count
    And fetching on any result set does not return a row
