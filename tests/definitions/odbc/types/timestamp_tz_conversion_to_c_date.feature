@odbc
Feature: ODBC TIMESTAMP_TZ to SQL_C_TYPE_DATE conversions

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_TYPE_DATE
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ with midnight UTC time is fetched as SQL_C_TYPE_DATE
    Then Date components are extracted from UTC without warning

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_TYPE_DATE boundary values
    Given Snowflake client is logged in
    When Boundary TIMESTAMP_TZ values are fetched as SQL_C_TYPE_DATE
    Then Date components match expected UTC values

  @odbc_e2e
  Scenario: TIMESTAMP_TZ to SQL_C_TYPE_DATE with time truncation
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ with non-zero UTC time is fetched as SQL_C_TYPE_DATE
    Then Date components are extracted from UTC with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIMESTAMP_TZ NULL to SQL_C_TYPE_DATE
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_TZ value is queried
    Then Indicator returns SQL_NULL_DATA
