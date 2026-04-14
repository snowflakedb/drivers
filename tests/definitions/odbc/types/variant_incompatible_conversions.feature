@odbc
Feature: ODBC VARIANT incompatible C type conversions

  @odbc_e2e
  Scenario: should fail converting VARIANT to numeric C types
    Given Snowflake client is logged in
    When A VARIANT value is fetched with numeric C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting VARIANT to temporal C types
    Given Snowflake client is logged in
    When A VARIANT value is fetched with temporal C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting VARIANT to interval C types
    Given Snowflake client is logged in
    When A VARIANT value is fetched with interval C types
    Then Each conversion should fail with SQLSTATE 07006

  @odbc_e2e
  Scenario: should fail converting VARIANT to SQL_C_GUID
    Given Snowflake client is logged in
    When A VARIANT value is fetched as SQL_C_GUID
    Then Conversion should fail with SQLSTATE 07006
