@odbc
Feature: ODBC DATE to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY
    Given Snowflake client is logged in
    When A DATE value is fetched as SQL_C_BINARY
    Then Raw bytes are returned with positive indicator

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY boundary values
    Given Snowflake client is logged in
    When Boundary DATE values (epoch, pre-epoch, leap day, end of year) are fetched as SQL_C_BINARY
    Then Raw bytes are returned with positive indicator

  @odbc_e2e
  Scenario: DATE to SQL_C_BINARY consistent size
    Given Snowflake client is logged in
    When Different DATE values are fetched as SQL_C_BINARY
    Then The indicator size is consistent across all dates

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
