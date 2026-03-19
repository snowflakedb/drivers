@odbc
Feature: Snowflake-specific statement attributes
  # Tests for SQL_SF_STMT_ATTR_LAST_QUERY_ID and SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_LAST_QUERY_ID returns empty string before any execution.
    Given Snowflake client is logged in
    When SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried on a fresh statement
    Then it should return SQL_SUCCESS and an empty string

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_LAST_QUERY_ID set returns HY092.
    Given Snowflake client is logged in
    When SQL_SF_STMT_ATTR_LAST_QUERY_ID is set to any value
    Then it should return SQL_ERROR with SQLSTATE HY092

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_LAST_QUERY_ID returns non-empty query ID after SQLExecDirect.
    Given Snowflake client is logged in
    When SQLExecDirect is called to execute a simple SELECT query
    And SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried
    Then it should return SQL_SUCCESS and a non-empty query ID string

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_LAST_QUERY_ID returns non-empty query ID after SQLPrepare and SQLExecute.
    Given Snowflake client is logged in
    When SQLPrepare and SQLExecute are called to execute a simple SELECT query
    And SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried
    Then it should return SQL_SUCCESS and a non-empty query ID string

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_LAST_QUERY_ID each execution produces a distinct query ID.
    Given Snowflake client is logged in
    When SQLExecDirect is called twice on the same statement
    Then each SQL_SF_STMT_ATTR_LAST_QUERY_ID value should be non-empty and different

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT default value is -1.
    Given Snowflake client is logged in
    When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value -1

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is set to 0, then 3, then reset to -1
    Then each get should return the value that was set

  @odbc_e2e
  Scenario: SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT invalid value less than -1 returns error.
    Given Snowflake client is logged in
    When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is set to -2
    Then it should return SQL_ERROR
