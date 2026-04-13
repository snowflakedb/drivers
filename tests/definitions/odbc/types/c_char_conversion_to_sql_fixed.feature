@odbc
Feature: ODBC C char types to SQL fixed-point conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR integer string to SQL_INTEGER
    Given Snowflake client is logged in
    When SQL_C_CHAR "42" is bound to SQL_INTEGER and inserted
    Then the value is read back as 42

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR negative integer string to SQL_BIGINT
    Given Snowflake client is logged in
    When SQL_C_CHAR "-9999999999" is bound to SQL_BIGINT and inserted
    Then the value is read back as -9999999999

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR decimal string to SQL_DECIMAL
    Given Snowflake client is logged in
    When SQL_C_CHAR "3.14" is bound to SQL_DECIMAL and inserted
    Then the value is read back as "3.14"

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR integer string to SQL_INTEGER
    Given Snowflake client is logged in
    When SQL_C_WCHAR "77" is bound to SQL_INTEGER and inserted
    Then the value is read back as 77

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_INTEGER
    Given Snowflake client is logged in
    When SQL_C_CHAR is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
