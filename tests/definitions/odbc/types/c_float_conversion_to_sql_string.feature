@odbc
Feature: ODBC C float types to SQL string conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_DOUBLE 3.14 is bound to SQL_VARCHAR and inserted
    Then the string representation contains 3.14

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_FLOAT 42.0 is bound to SQL_VARCHAR and inserted
    Then the string representation contains 42

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE with NULL indicator to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_DOUBLE is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
