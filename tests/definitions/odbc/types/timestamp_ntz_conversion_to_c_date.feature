@odbc
Feature: ODBC TIMESTAMP_NTZ to SQL_C_TYPE_DATE conversions

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_TYPE_DATE
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ with midnight time is fetched as SQL_C_TYPE_DATE
    Then Date components are extracted without warning

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_TYPE_DATE boundary values
    Given Snowflake client is logged in
    When Boundary TIMESTAMP_NTZ values are fetched as SQL_C_TYPE_DATE
    Then Date components match expected values

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ to SQL_C_TYPE_DATE with time truncation
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ with non-zero time is fetched as SQL_C_TYPE_DATE
    Then Date components are extracted with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ NULL to SQL_C_TYPE_DATE
    Given Snowflake client is logged in
    When A NULL TIMESTAMP_NTZ value is queried
    Then Indicator returns SQL_NULL_DATA
