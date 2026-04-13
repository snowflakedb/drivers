@odbc
Feature: ODBC C date type to SQL date conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE to SQL_TYPE_DATE and read back
    Given Snowflake client is logged in
    When A date struct is bound to SQL_TYPE_DATE and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE with NULL indicator to SQL_TYPE_DATE
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
