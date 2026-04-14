@odbc
Feature: ODBC ARRAY to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: ARRAY to SQL_C_BINARY
    Given Snowflake client is logged in
    When An ARRAY value is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as array with correct element count

  @odbc_e2e
  Scenario: ARRAY to SQL_C_BINARY empty
    Given Snowflake client is logged in
    When An empty ARRAY is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as empty array

  @odbc_e2e
  Scenario: ARRAY to SQL_C_BINARY nested
    Given Snowflake client is logged in
    When A nested ARRAY is fetched as SQL_C_BINARY
    Then Raw JSON bytes are returned and parseable as nested array

  @odbc_e2e
  Scenario: ARRAY to SQL_C_BINARY buffer too small
    Given Snowflake client is logged in
    When An ARRAY value is fetched into a buffer smaller than the JSON representation
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: ARRAY to SQL_C_BINARY exact buffer fit
    Given Snowflake client is logged in
    When An ARRAY value is fetched into an exact-size buffer
    Then The indicator equals the buffer size used and the data is valid JSON

  @odbc_e2e
  Scenario: ARRAY NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL ARRAY value is queried
    Then Indicator returns SQL_NULL_DATA
