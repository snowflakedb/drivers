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
  Scenario: OBJECT to SQL_C_BINARY buffer too small
    Given Snowflake client is logged in
    When An OBJECT value is fetched into a buffer smaller than the JSON representation
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: OBJECT to SQL_C_BINARY exact buffer fit
    Given Snowflake client is logged in
    When An OBJECT value is fetched into an exact-size buffer
    Then The indicator equals the buffer size used and the data is valid JSON

  @odbc_e2e
  Scenario: OBJECT NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL OBJECT value is queried
    Then Indicator returns SQL_NULL_DATA
