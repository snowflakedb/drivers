@odbc
Feature: ODBC DATE to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY
    Given Snowflake client is logged in
    When A DATE value is fetched as SQL_C_BINARY
    Then SQL_DATE_STRUCT fields match the source date

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY struct field verification
    Given Snowflake client is logged in
    When epoch DATE is fetched as SQL_C_BINARY
    Then SQL_DATE_STRUCT year, month, day fields match
    When pre-epoch DATE is fetched as SQL_C_BINARY
    Then SQL_DATE_STRUCT year, month, day fields match
    When leap day DATE is fetched as SQL_C_BINARY
    Then SQL_DATE_STRUCT year, month, day fields match
    When end of year DATE is fetched as SQL_C_BINARY
    Then SQL_DATE_STRUCT year, month, day fields match
    When far future DATE is fetched as SQL_C_BINARY
    Then SQL_DATE_STRUCT year, month, day fields match
    When far past DATE is fetched as SQL_C_BINARY
    Then SQL_DATE_STRUCT year, month, day fields match

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY exact buffer fit
    Given Snowflake client is logged in
    When A DATE value is fetched into a buffer of exactly sizeof(SQL_DATE_STRUCT)
    Then SQL_SUCCESS is returned with correct struct fields

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY buffer too small
    # BD#42: Old driver does not return 22003 for undersized DATE binary buffer
    Given Snowflake client is logged in
    When A DATE value is fetched into a buffer smaller than sizeof(SQL_DATE_STRUCT)
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY consistent size
    Given Snowflake client is logged in
    When Different DATE values are fetched as SQL_C_BINARY
    Then The indicator equals sizeof(SQL_DATE_STRUCT) for all dates

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
