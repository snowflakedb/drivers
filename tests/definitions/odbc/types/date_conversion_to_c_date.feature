@odbc
Feature: ODBC DATE to SQL_C_TYPE_DATE conversions

  @odbc_e2e
  Scenario: DATE to SQL_C_TYPE_DATE
    Given Snowflake client is logged in
    When A DATE value is fetched as SQL_C_TYPE_DATE
    Then Date components match the source value

  @odbc_e2e
  Scenario: DATE to SQL_C_TYPE_DATE boundary values
    Given Snowflake client is logged in
    When pre-epoch DATE is fetched as SQL_C_TYPE_DATE
    Then Date components match expected values
    When Leap day DATE is fetched as SQL_C_TYPE_DATE
    Then Date components match expected values
    When End-of-year DATE is fetched as SQL_C_TYPE_DATE
    Then Date components match expected values
    When Epoch DATE is fetched as SQL_C_TYPE_DATE
    Then Date components match expected values

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_TYPE_DATE
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: DATE to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A DATE value is fetched as SQL_C_DEFAULT
    Then SQL_C_DEFAULT resolves to SQL_C_TYPE_DATE with correct values

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
