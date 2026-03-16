@odbc
Feature: Attribute error handling
  # Tests for ODBC spec compliance: HY090 on negative buffer length,
  # SQL_SUCCESS_WITH_INFO on string truncation.

  @odbc_e2e
  Scenario: SQLGetStmtAttr with negative buffer length returns HY090.
    Given Snowflake client is logged in
    When SQLGetStmtAttr is called with buffer_length = -1 for a string attribute
    Then it should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLGetStmtAttr string attribute truncation returns SQL_SUCCESS_WITH_INFO.
    Given Snowflake client is logged in
    When SQLExecDirect is called and SQLGetStmtAttr is called with an insufficient buffer
    Then it should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004

  @odbc_e2e
  Scenario: SQLGetConnectAttr with negative buffer length returns HY090.
    Given Snowflake client is logged in
    When SQLGetConnectAttr is called with buffer_length = -1 for a string attribute
    Then it should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLGetStmtAttr with invalid attribute identifier returns HY092.
    Given Snowflake client is logged in
    When SQLGetStmtAttr is called with an invalid attribute identifier
    Then it should return SQL_ERROR with SQLSTATE HY092
