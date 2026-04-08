@odbc
Feature: ODBC SQLBindParameter C float/double types to SQL_BIT conversion
  # Tests for binding SQL_C_DOUBLE and SQL_C_FLOAT values to SQL_BIT
  # (boolean) parameters.

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE nonzero to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be TRUE

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE zero to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be FALSE

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE negative to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be TRUE (negative nonzero)

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT nonzero to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be TRUE

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT zero to SQL_BIT.
    Given Snowflake client is logged in
    When the C type value is bound as SQL_BIT and SELECT ? is executed
    Then the result should be FALSE
