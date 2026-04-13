@odbc
Feature: ODBC C timestamp type to SQL timestamp conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIMESTAMP and read back
    Given Snowflake client is logged in
    When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 is bound and inserted
    Then the value contains the date and time components

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
