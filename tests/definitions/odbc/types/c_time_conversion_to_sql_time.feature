@odbc
Feature: ODBC C time type to SQL time conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME to SQL_TYPE_TIME and read back
    Given Snowflake client is logged in
    When A time struct is bound to SQL_TYPE_TIME and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME with NULL indicator to SQL_TYPE_TIME
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
