@odbc
Feature: ODBC C char types to SQL real conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR float string to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_CHAR "3.14" is bound to SQL_DOUBLE and inserted
    Then the value is read back as approximately 3.14

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR integer string to SQL_REAL
    Given Snowflake client is logged in
    When SQL_C_CHAR "100" is bound to SQL_REAL and inserted
    Then the value is read back as 100.0

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR float string to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_WCHAR "2.71" is bound to SQL_DOUBLE and inserted
    Then the value is read back as approximately 2.71

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_CHAR is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
