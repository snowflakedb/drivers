@odbc
Feature: ODBC SQLBindParameter C char types to SQL_BIT conversion
  # Tests for binding SQL_C_CHAR and SQL_C_WCHAR string values to SQL_BIT
  # (boolean) parameters.

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR '1' to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be TRUE

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR '0' to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be FALSE

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR '1' to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be TRUE

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR '0' to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be FALSE
