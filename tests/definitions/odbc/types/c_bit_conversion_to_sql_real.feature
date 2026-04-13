@odbc
Feature: ODBC C bit type to SQL real conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_BIT one to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_BIT 1 is bound to SQL_DOUBLE and inserted
    Then the value is read back as 1.0

  @odbc_e2e
  Scenario: should bind SQL_C_BIT zero to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_BIT 0 is bound to SQL_DOUBLE and inserted
    Then the value is read back as 0.0

  @odbc_e2e
  Scenario: should bind SQL_C_BIT to SQL_REAL
    Given Snowflake client is logged in
    When SQL_C_BIT 1 is bound to SQL_REAL and inserted
    Then the value is read back as 1.0
