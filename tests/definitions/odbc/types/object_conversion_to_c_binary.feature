@odbc
Feature: ODBC OBJECT to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: OBJECT to SQL_C_BINARY
    Given Snowflake client is logged in
    When An OBJECT value is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as object

  @odbc_e2e
  Scenario: OBJECT to SQL_C_BINARY empty
    Given Snowflake client is logged in
    When An empty OBJECT is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as empty object

  @odbc_e2e
  Scenario: OBJECT to SQL_C_BINARY nested
    Given Snowflake client is logged in
    When A nested OBJECT is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable with nested structure intact

  @odbc_e2e
  Scenario: OBJECT NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL OBJECT value is queried
    Then Indicator returns SQL_NULL_DATA
