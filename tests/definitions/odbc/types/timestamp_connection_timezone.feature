@odbc
Feature: ODBC TIMESTAMP connection-level timezone behavior

  @odbc_e2e
  Scenario: Connection-level TIMEZONE parameter via DSN
    Given Snowflake client is logged in
    When Session timezone is verified via SHOW PARAMETERS
    Then A valid timezone string is returned

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ literal interpretation changes with session timezone
    Given Snowflake client is logged in
    When A bare literal is cast to TIMESTAMP_LTZ under different session timezones
    Then The resulting UTC epoch differs because the literal is interpreted in the session timezone

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ is unaffected by session timezone changes
    Given Snowflake client is logged in
    When The same NTZ literal is fetched with different session timezones
    Then The struct values are identical because NTZ has no timezone semantics
