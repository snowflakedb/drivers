@odbc
Feature: ODBC C char types to SQL boolean conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR true string to SQL_BIT
    Given Snowflake client is logged in
    When A character string 1 is bound to SQL_BIT and inserted
    Then The value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR false string to SQL_BIT
    Given Snowflake client is logged in
    When A character string 0 is bound to SQL_BIT and inserted
    Then The value is read back as SQL_C_BIT 0

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR true string to SQL_BIT
    Given Snowflake client is logged in
    When A wide character string 1 is bound to SQL_BIT and inserted
    Then The value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_BIT
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
