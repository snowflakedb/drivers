@odbc
Feature: ODBC C timestamp type to SQL timestamp conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIMESTAMP and read back
    Given Snowflake client is logged in
    When A timestamp struct is bound to SQL_TYPE_TIMESTAMP and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
