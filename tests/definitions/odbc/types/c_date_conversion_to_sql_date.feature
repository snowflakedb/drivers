@odbc
Feature: ODBC C date type to SQL date conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE to SQL_TYPE_DATE and read back
    Given Snowflake client is logged in
    When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_DATE and inserted
    Then the value is read back as 2026-04-13

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE with NULL indicator to SQL_TYPE_DATE
    Given Snowflake client is logged in
    When SQL_C_TYPE_DATE is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
