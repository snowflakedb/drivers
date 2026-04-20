@odbc
Feature: ODBC OBJECT incompatible C type conversions

  @odbc_e2e
  Scenario: should fail converting OBJECT to numeric C types
    Given Snowflake client is logged in
    When An OBJECT value is fetched with numeric C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting OBJECT to temporal C types
    Given Snowflake client is logged in
    When An OBJECT value is fetched with temporal C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting OBJECT to interval C types
    Given Snowflake client is logged in
    When An OBJECT value is fetched with interval C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting OBJECT to SQL_C_GUID
    Given Snowflake client is logged in
    When An OBJECT value is fetched as SQL_C_GUID
    Then Conversion should fail with SQLSTATE 07006
