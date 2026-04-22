@odbc
Feature: ODBC TIMESTAMP_NTZ to SQL_C_TYPE_TIME conversions

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_TYPE_TIME
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ with zero fractional seconds is fetched as SQL_C_TYPE_TIME
    Then Time components are extracted without warning

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_TYPE_TIME midnight
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ with midnight time is fetched as SQL_C_TYPE_TIME
    Then All time components are zero

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_TYPE_TIME with fractional truncation
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ with non-zero fractional seconds is fetched as SQL_C_TYPE_TIME
    Then Time components are extracted with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ NULL to SQL_C_TYPE_TIME
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_NTZ value is queried
    Then Indicator returns SQL_NULL_DATA
