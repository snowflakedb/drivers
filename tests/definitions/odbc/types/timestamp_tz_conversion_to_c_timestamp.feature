@odbc
Feature: ODBC TIMESTAMP_TZ to SQL_C_TYPE_TIMESTAMP conversions

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_TYPE_TIMESTAMP basic values
    Given Snowflake client is logged in
    When TIMESTAMP_TZ values are fetched as SQL_C_TYPE_TIMESTAMP
    Then SQL_TIMESTAMP_STRUCT fields match the UTC representation of the timestamp

  @odbc_e2e
  Scenario: TIMESTAMP_TZ NULL to SQL_C_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_TZ value is queried
    Then Indicator returns SQL_NULL_DATA
