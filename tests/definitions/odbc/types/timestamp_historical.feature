@odbc
Feature: ODBC TIMESTAMP historical and far-future era conversions

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ historical era year 1600
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ from year 1600 is fetched
    Then SQL_TIMESTAMP_STRUCT contains the correct historical date

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ far future year 3017
    Given Snowflake client is logged in
    When A TIMESTAMP_NTZ from year 3017 is fetched
    Then SQL_TIMESTAMP_STRUCT contains the correct far-future date

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ year 0001 minimum representable date
    Given Snowflake client is logged in
    When The earliest possible Snowflake timestamp is fetched
    Then SQL_TIMESTAMP_STRUCT contains year 1

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ year 9999 maximum representable date
    Given Snowflake client is logged in
    When The latest possible Snowflake timestamp is fetched
    Then SQL_TIMESTAMP_STRUCT contains the maximum date

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ historical era year 1600
    Given Snowflake client is logged in with UTC timezone
    When A TIMESTAMP_LTZ from year 1600 is fetched
    Then SQL_TIMESTAMP_STRUCT contains the correct historical date

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ far future year 3017
    Given Snowflake client is logged in with UTC timezone
    When A TIMESTAMP_LTZ from year 3017 is fetched
    Then SQL_TIMESTAMP_STRUCT contains the correct far-future date

  @odbc_e2e
  Scenario: TIMESTAMP_TZ historical era year 1600 with offset crossing date boundary
    Given Snowflake client is logged in
    When A pre-epoch TIMESTAMP_TZ with offset crossing a date boundary is fetched
    Then SQL_TIMESTAMP_STRUCT contains the UTC-converted date (crosses back to previous day)

  @odbc_e2e
  Scenario: TIMESTAMP_TZ far future year 3017
    Given Snowflake client is logged in
    When A TIMESTAMP_TZ from year 3017 is fetched
    Then SQL_TIMESTAMP_STRUCT contains the UTC-converted far-future date

  @odbc_e2e
  Scenario: TIMESTAMP_TZ year 0001 with positive offset
    Given Snowflake client is logged in
    When The earliest representable TIMESTAMP_TZ with a positive offset is fetched
    Then SQL_TIMESTAMP_STRUCT contains the UTC-converted value

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ historical dates as SQL_C_CHAR
    Given Snowflake client is logged in
    When Historical and far-future timestamps are fetched as SQL_C_CHAR
    Then String representations are correctly formatted
