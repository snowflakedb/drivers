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
    When pre-epoch DATE is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields are populated and time fields are zero
    When leap day DATE is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields are populated and time fields are zero
    When epoch DATE is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields are populated and time fields are zero
    When end of year DATE is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields are populated and time fields are zero
    When first day of year DATE is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields are populated and time fields are zero

  @odbc_e2e
  Scenario: DATE to SQL_C_TYPE_TIMESTAMP far future
    Given Snowflake client is logged in
    When The maximum DATE value 9999-12-31 is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields match 9999-12-31 and time fields are zero

  @odbc_e2e
  Scenario: DATE to SQL_C_TYPE_TIMESTAMP far past
    Given Snowflake client is logged in
    When The minimum DATE value 0001-01-01 is fetched as SQL_C_TYPE_TIMESTAMP
    Then Date fields match 0001-01-01 and time fields are zero

  @odbc_e2e
  Scenario: DATE NULL to SQL_C_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A NULL DATE value is queried
    Then Indicator returns SQL_NULL_DATA
