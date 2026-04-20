@odbc
Feature: ODBC ARRAY incompatible C type conversions

  @odbc_e2e
  Scenario: should fail converting ARRAY to numeric C types
    Given Snowflake client is logged in
    When An ARRAY value is fetched with numeric C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting ARRAY to temporal C types
    Given Snowflake client is logged in
    When An ARRAY value is fetched with temporal C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting ARRAY to interval C types
    Given Snowflake client is logged in
    When An ARRAY value is fetched with interval C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting ARRAY to SQL_C_GUID
    Given Snowflake client is logged in
    When An ARRAY value is fetched as SQL_C_GUID
    Then Conversion should fail with SQLSTATE 07006
