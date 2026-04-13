@odbc
Feature: ODBC C char types to SQL boolean conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR true string to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_CHAR "1" is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR false string to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_CHAR "0" is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 0

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR true string to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_WCHAR "1" is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_CHAR is bound with SQL_NULL_DATA to SQL_BIT and inserted
    Then the stored value should be NULL
