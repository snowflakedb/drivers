@odbc
Feature: ODBC SQLBindParameter SQL_C_BIT to SQL_BIT conversion
  # Tests for binding SQL_C_BIT values to SQL_BIT (boolean) parameters.

  @odbc_e2e
  Scenario: should bind SQL_C_BIT true to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be TRUE

  @odbc_e2e
  Scenario: should bind SQL_C_BIT false to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be FALSE

  @odbc_e2e
  Scenario: should bind SQL_C_BIT nonzero value to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be TRUE (any nonzero value)
