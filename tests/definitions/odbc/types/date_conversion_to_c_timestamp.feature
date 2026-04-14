@odbc
Feature: ODBC DATE to SQL_C_TYPE_TIMESTAMP conversions

  @odbc_e2e
  Scenario: DATE to SQL_C_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A DATE value is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields are populated and time fields are zero

  @odbc_e2e
  Scenario: DATE to SQL_C_TYPE_TIMESTAMP boundary values
    Given Snowflake client is logged in
    When Boundary DATE values (pre-epoch, leap day, epoch, end of year, first of year) are fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields are populated and time fields are zero for each

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
