@odbc
Feature: ODBC TIME to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY
    # BD#43: Old driver does not support TIME to SQL_C_BINARY conversion
    Given Snowflake client is logged in
    When A TIME value is fetched as SQL_C_BINARY
    Then SQL_TIME_STRUCT fields match the source time

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY struct field verification
    # BD#43: Old driver does not support TIME to SQL_C_BINARY conversion
    Given Snowflake client is logged in
    When midnight TIME is fetched as SQL_C_BINARY
    Then SQL_TIME_STRUCT fields match
    When end of day TIME is fetched as SQL_C_BINARY
    Then SQL_TIME_STRUCT fields match
    When single-digit TIME is fetched as SQL_C_BINARY
    Then SQL_TIME_STRUCT fields match
    When fractional TIME is fetched as SQL_C_BINARY
    Then SQL_TIME_STRUCT fields match (fractional seconds dropped)

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY exact buffer fit
    # BD#43: Old driver does not support TIME to SQL_C_BINARY conversion
    Given Snowflake client is logged in
    When A TIME value is fetched into a buffer of exactly sizeof(SQL_TIME_STRUCT)
    Then SQL_SUCCESS is returned with correct struct fields

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY buffer too small
    # BD#43: Old driver does not support TIME to SQL_C_BINARY conversion
    Given Snowflake client is logged in
    When A TIME value is fetched into a buffer smaller than sizeof(SQL_TIME_STRUCT)
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY consistent size
    # BD#43: Old driver does not support TIME to SQL_C_BINARY conversion
    Given Snowflake client is logged in
    When Different TIME values are fetched as SQL_C_BINARY
    Then The indicator equals sizeof(SQL_TIME_STRUCT) for all times

  @odbc_e2e
  Scenario: TIME NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL TIME value is queried
    Then Indicator returns SQL_NULL_DATA
