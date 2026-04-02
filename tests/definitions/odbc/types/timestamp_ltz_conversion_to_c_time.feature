@odbc
Feature: ODBC TIMESTAMP_LTZ to SQL_C_TYPE_TIME conversions

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_TYPE_TIME
    Given Snowflake client is logged in with a known session timezone
    When A TIMESTAMP_LTZ with zero fractional seconds is fetched as SQL_C_TYPE_TIME
    Then Time components are extracted without warning

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_TYPE_TIME midnight
    Given Snowflake client is logged in with a known session timezone
    When A TIMESTAMP_LTZ with midnight time is fetched as SQL_C_TYPE_TIME
    Then All time components are zero

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_TYPE_TIME with fractional truncation
    Given Snowflake client is logged in with a known session timezone
    When A TIMESTAMP_LTZ with non-zero fractional seconds is fetched as SQL_C_TYPE_TIME
    Then Time components are extracted with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ NULL to SQL_C_TYPE_TIME
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_LTZ value is queried
    Then Indicator returns SQL_NULL_DATA
