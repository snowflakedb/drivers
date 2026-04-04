@odbc
Feature: ODBC TIMESTAMP_LTZ to SQL_C_TYPE_DATE conversions

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_TYPE_DATE
    Given Snowflake client is logged in with a known session timezone
    When A TIMESTAMP_LTZ with midnight time is fetched as SQL_C_TYPE_DATE
    Then Date components are extracted without warning

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_TYPE_DATE boundary values
    Given Snowflake client is logged in with a known session timezone
    When Boundary TIMESTAMP_LTZ values are fetched as SQL_C_TYPE_DATE
    Then Date components match expected values

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ to SQL_C_TYPE_DATE with time truncation
    Given Snowflake client is logged in with a known session timezone
    When A TIMESTAMP_LTZ with non-zero time is fetched as SQL_C_TYPE_DATE
    Then Date components are extracted with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ NULL to SQL_C_TYPE_DATE
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_LTZ value is queried
    Then Indicator returns SQL_NULL_DATA
