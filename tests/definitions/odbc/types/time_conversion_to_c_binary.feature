@odbc
Feature: ODBC TIME to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY
    Given Snowflake client is logged in
    When A TIME value is fetched as SQL_C_BINARY
    Then Raw bytes are returned with positive indicator

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY midnight
    Given Snowflake client is logged in
    When Midnight TIME is fetched as SQL_C_BINARY
    Then Raw bytes are returned with positive indicator

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY end of day
    Given Snowflake client is logged in
    When End-of-day TIME is fetched as SQL_C_BINARY
    Then Raw bytes are returned with positive indicator

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY with fractional seconds
    Given Snowflake client is logged in
    When A TIME with fractional seconds is fetched as SQL_C_BINARY
    Then Raw bytes are returned with positive indicator

  @odbc_e2e
  Scenario: TIME to SQL_C_BINARY consistent size
    Given Snowflake client is logged in
    When Different TIME values are fetched as SQL_C_BINARY
    Then The indicator size is consistent across all times

  @odbc_e2e
  Scenario: TIME NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL TIME value is queried
    Then Indicator returns SQL_NULL_DATA
