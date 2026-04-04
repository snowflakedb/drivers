@odbc
Feature: ODBC TIMESTAMP_NTZ to SQL_C_TYPE_TIMESTAMP conversions

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_TYPE_TIMESTAMP basic values
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ values are fetched as SQL_C_TYPE_TIMESTAMP
    Then SQL_TIMESTAMP_STRUCT fields match expected date and time components

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ NULL to SQL_C_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_NTZ value is queried
    Then Indicator returns SQL_NULL_DATA
