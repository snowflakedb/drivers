@odbc
Feature: ODBC C float types to SQL string conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_DOUBLE is bound to SQL_VARCHAR and inserted
    Then The string representation is read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_FLOAT is bound to SQL_VARCHAR and inserted
    Then The string representation is read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE with NULL indicator to SQL_VARCHAR
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
