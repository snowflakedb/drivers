@odbc
Feature: ODBC C char types to SQL string conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When A character string is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR empty string to SQL_VARCHAR
    Given Snowflake client is logged in
    When An empty string is bound to SQL_VARCHAR and inserted
    Then The value should be read back as empty

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When A wide character string is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_VARCHAR
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
