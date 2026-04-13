@odbc
Feature: ODBC C float types to SQL boolean conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE one to SQL_BIT via float
    Given Snowflake client is logged in
    When SQL_C_DOUBLE 1.0 is bound to SQL_BIT and inserted
    Then The value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE zero to SQL_BIT via float
    Given Snowflake client is logged in
    When SQL_C_DOUBLE 0.0 is bound to SQL_BIT and inserted
    Then The value is read back as SQL_C_BIT 0

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_FLOAT 1.0 is bound to SQL_BIT and inserted
    Then The value is read back as SQL_C_BIT 1
