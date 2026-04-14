@odbc
Feature: ODBC TIME to SQL_C_TYPE_TIMESTAMP conversions

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A TIME value is fetched as SQL_C_TYPE_TIMESTAMP
    Then Time fields are populated and date fields are set to current date

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIMESTAMP midnight
    Given Snowflake client is logged in
    When Midnight TIME is fetched as SQL_C_TYPE_TIMESTAMP
    Then All time components are zero and date is current date

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIMESTAMP end of day
    Given Snowflake client is logged in
    When End-of-day TIME is fetched as SQL_C_TYPE_TIMESTAMP
    Then Time components match 23:59:59 and date is current date

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIMESTAMP with fractional truncation
    # BD#42: Old driver does not report 01S07 for fractional seconds
    Given Snowflake client is logged in
    When A TIME with non-zero fractional seconds is fetched as SQL_C_TYPE_TIMESTAMP
    Then Time components are extracted with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIME to SQL_C_TYPE_TIMESTAMP with high-precision fractional truncation
    # BD#42: Old driver does not report 01S07 for fractional seconds
    Given Snowflake client is logged in
    When A TIME with high-precision fractional seconds is fetched as SQL_C_TYPE_TIMESTAMP
    Then Time components are extracted with SQLSTATE 01S07 warning

  @odbc_e2e
  Scenario: TIME NULL to SQL_C_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When A NULL TIME value is queried
    Then Indicator returns SQL_NULL_DATA
