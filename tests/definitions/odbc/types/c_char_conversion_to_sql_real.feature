@odbc
Feature: ODBC C char types to SQL real conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR float string to SQL_DOUBLE
    Given Snowflake client is logged in
    When A character float string is bound to SQL_DOUBLE and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR integer string to SQL_REAL
    Given Snowflake client is logged in
    When A character integer string is bound to SQL_REAL and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR float string to SQL_DOUBLE
    Given Snowflake client is logged in
    When A wide character float string is bound to SQL_DOUBLE and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_DOUBLE
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
