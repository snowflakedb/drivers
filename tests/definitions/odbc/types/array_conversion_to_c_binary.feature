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
  Scenario: ARRAY NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL ARRAY value is queried
    Then Indicator returns SQL_NULL_DATA
