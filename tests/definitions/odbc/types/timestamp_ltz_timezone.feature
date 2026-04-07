@odbc
Feature: ODBC TIMESTAMP_LTZ timezone-aware conversions

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ with America/New_York timezone
    Given Snowflake client is logged in with America/New_York timezone
    When A known UTC instant is fetched as TIMESTAMP_LTZ via SQL_C_TYPE_TIMESTAMP
    Then SQL_TIMESTAMP_STRUCT contains UTC time (struct has no timezone field)

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ with Asia/Kolkata timezone
    Given Snowflake client is logged in with Asia/Kolkata timezone (UTC+5:30)
    When A known UTC instant is fetched as TIMESTAMP_LTZ via SQL_C_TYPE_TIMESTAMP
    Then SQL_TIMESTAMP_STRUCT contains UTC time (hour 18, not IST hour 0 next day)

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ with Pacific/Auckland timezone
    Given Snowflake client is logged in with Pacific/Auckland timezone (UTC+12 or +13 DST)
    When A known UTC instant is fetched as TIMESTAMP_LTZ via SQL_C_TYPE_TIMESTAMP
    Then SQL_TIMESTAMP_STRUCT contains UTC time (hour 10, not NZDT hour 23)

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ timezone does not affect SQL_C_TYPE_TIMESTAMP struct
    Given Snowflake client is logged in
    When The same UTC instant is fetched with different session timezones
    Then SQL_TIMESTAMP_STRUCT always contains the same UTC values

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ timezone does not affect SQL_C_CHAR output
    Given Snowflake client is logged in
    When The same UTC instant is fetched as SQL_C_CHAR with different session timezones
    Then String output is the same regardless of session timezone

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ implicit timezone from literal without offset
    Given Snowflake client is logged in with two different timezones
    When A TIMESTAMP_LTZ literal without explicit offset is cast
    Then The session timezone determines the UTC instant, so struct values differ
