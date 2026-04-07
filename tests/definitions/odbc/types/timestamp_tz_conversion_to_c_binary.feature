@odbc
Feature: ODBC TIMESTAMP_TZ to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_BINARY
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ value is fetched as SQL_C_BINARY with sufficient buffer
    Then SQL_SUCCESS is returned and indicator equals sizeof(SQL_TIMESTAMP_STRUCT)

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_BINARY with fractional seconds
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ with fractional seconds is fetched as SQL_C_BINARY
    Then SQL_SUCCESS is returned and fraction field is preserved

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_BINARY pre-epoch
    Given Snowflake client is logged in
    When A pre-epoch TIMESTAMP_TZ value is fetched as SQL_C_BINARY
    Then SQL_SUCCESS is returned with correct pre-epoch date components

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_BINARY buffer too small
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ value is fetched into a buffer smaller than sizeof(SQL_TIMESTAMP_STRUCT)
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIMESTAMP_TZ NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_TZ value is queried
    Then Indicator returns SQL_NULL_DATA
