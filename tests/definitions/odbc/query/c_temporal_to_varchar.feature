@odbc
Feature: ODBC SQLBindParameter C temporal types to VARCHAR conversion
  # Tests for binding SQL_C_TYPE_TIMESTAMP, SQL_C_TYPE_DATE, SQL_C_TYPE_TIME
  # to SQL_VARCHAR.

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP with fraction to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string
