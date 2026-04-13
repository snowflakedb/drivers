@odbc
Feature: ODBC C char types to SQL fixed-point conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR integer string to SQL_INTEGER
    Given Snowflake client is logged in
    When A character integer string is bound to SQL_INTEGER and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR negative integer string to SQL_BIGINT
    Given Snowflake client is logged in
    When A negative integer string is bound to SQL_BIGINT and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR decimal string to SQL_DECIMAL
    Given Snowflake client is logged in
    When A decimal string is bound to SQL_DECIMAL and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR integer string to SQL_INTEGER
    Given Snowflake client is logged in
    When A wide character integer string is bound to SQL_INTEGER and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_INTEGER
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
