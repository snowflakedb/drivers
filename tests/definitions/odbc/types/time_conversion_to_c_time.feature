@odbc
Feature: ODBC TIME to SQL_C_TYPE_TIME conversions

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIME
    Given Snowflake client is logged in
    When A TIME with zero fractional seconds is fetched as SQL_C_TYPE_TIME
    Then Time components are extracted without warning

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIME midnight
    Given Snowflake client is logged in
    When A TIME with midnight value is fetched as SQL_C_TYPE_TIME
    Then All time components are zero

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIME end of day
    Given Snowflake client is logged in
    When A TIME near end of day is fetched as SQL_C_TYPE_TIME
    Then Time components match end of day values

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIME with fractional truncation
    Given Snowflake client is logged in
    When A TIME with non-zero fractional seconds is fetched as SQL_C_TYPE_TIME
    Then Time components are extracted with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIME NULL to SQL_C_TYPE_TIME
    Given Snowflake client is logged in
    When A NULL TIME value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: TIME to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A TIME with zero fractional seconds is fetched as SQL_C_DEFAULT
    Then SQL_C_DEFAULT resolves to SQL_C_TYPE_TIME with correct values

  @odbc_e2e
  Scenario: TIME to SQL_C_DEFAULT with fractional truncation
    Given Snowflake client is logged in
    When A TIME with non-zero fractional seconds is fetched as SQL_C_DEFAULT
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01S07

  @odbc_e2e
  Scenario: TIME NULL to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When A NULL TIME value is queried
    Then Indicator returns SQL_NULL_DATA
