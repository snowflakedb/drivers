@odbc
Feature: ODBC SQLBindParameter C char/default types to VARCHAR conversion
  # Tests for SQL_C_DEFAULT to SQL_VARCHAR and verifying all string SQL types
  # (SQL_CHAR, SQL_LONGVARCHAR, SQL_WCHAR, SQL_WVARCHAR, SQL_WLONGVARCHAR)
  # route through the same conversion.

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR with SQL_NTS to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound with SQL_NTS indicator and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_DEFAULT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_CHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_LONGVARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_WCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_WVARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_WLONGVARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string
