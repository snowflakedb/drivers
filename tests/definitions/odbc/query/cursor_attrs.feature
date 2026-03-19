@odbc
Feature: Cursor statement attributes
  # Tests for SQL_ATTR_ROW_NUMBER and SQL_ATTR_ROW_OPERATION_PTR

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_NUMBER default value is 0.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_NUMBER is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value 0

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_NUMBER is read-only.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_NUMBER is set on a statement
    Then it should return SQL_ERROR with HY092

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_NUMBER increments on each SQLFetch call.
    Given Snowflake client is logged in
    When SQLFetch is called repeatedly on a result set
    Then SQL_ATTR_ROW_NUMBER should increment by 1 on each call

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_NUMBER resets to 0 after all rows are fetched.
    Given Snowflake client is logged in
    When all rows have been fetched from a result set
    Then SQL_ATTR_ROW_NUMBER should be 0

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_OPERATION_PTR default value is NULL.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_OPERATION_PTR is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value NULL

  @odbc_e2e
  Scenario: SQL_ATTR_ROW_OPERATION_PTR can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_ATTR_ROW_OPERATION_PTR is set to a pointer
    Then it should return SQL_SUCCESS and the retrieved pointer should match
