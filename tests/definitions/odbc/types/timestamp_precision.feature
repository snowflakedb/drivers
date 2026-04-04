@odbc
Feature: ODBC TIMESTAMP precision variant conversions

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ precision 0 truncates fractional seconds
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ(0) is queried with a fractional-second value
    Then Fractional part is zero because precision 0 truncates sub-second digits

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ precision 3 keeps milliseconds
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ(3) is queried with nanosecond-precision input
    Then Only millisecond precision is preserved (3 decimal digits, truncated)

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ precision 6 keeps microseconds
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ(6) is queried with nanosecond-precision input
    Then Only microsecond precision is preserved (6 decimal digits)

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ precision 9 keeps nanoseconds
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ(9) is queried with full nanosecond-precision input
    Then Full nanosecond precision is preserved

  @odbc_e2e
  Scenario: TIMESTAMP_LTZ precision variants
    Given Snowflake client is logged in with UTC timezone
    When TIMESTAMP_LTZ values at different precisions are fetched
    Then Fractional seconds are truncated according to declared precision

  @odbc_e2e
  Scenario: TIMESTAMP_TZ precision variants
    Given Snowflake client is logged in
    When TIMESTAMP_TZ values at different precisions are fetched
    Then Fractional seconds are truncated according to declared precision

  @odbc_e2e
  Scenario: TIMESTAMP_NTZ precision affects SQL_C_CHAR output
    Given Snowflake client is logged in
    When TIMESTAMP_NTZ at various precisions is fetched as SQL_C_CHAR
    Then String representation reflects the declared precision
