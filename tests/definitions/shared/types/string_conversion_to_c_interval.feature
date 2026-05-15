@odbc @core_not_needed @python_not_needed @jdbc_not_needed
Feature: VARCHAR -> SQL_C_INTERVAL_* fetch (SQLGetData / SQLBindCol)
  # Snowflake holds interval values as VARCHAR ANSI-literal text. When such a
  # column is fetched as a SQL_C_INTERVAL_* C type the driver parses the
  # literal per ODBC Appendix D ("Character to Interval") and writes a
  # SQL_INTERVAL_STRUCT. The four spec outcomes are:
  #
  #   1. Valid value, no truncation                 -> SQL_SUCCESS
  #   2. Valid value, truncation of trailing fields -> SQL_SUCCESS_WITH_INFO, 01S07
  #   3. Valid value, leading-field precision lost  -> SQL_ERROR, 22015
  #   4. Not a valid interval value                 -> SQL_ERROR, 22018
  #
  # The reference (old) ODBC driver does not implement this conversion; it
  # rejects every SQL_C_INTERVAL_* target with SQLSTATE 07006. The new
  # behavior is captured in BD#54.

  # ============================================================================
  # OUTCOME #1 - SQL_SUCCESS
  # ============================================================================

  @odbc_e2e
  Scenario: should fetch VARCHAR as single-field SQL_C_INTERVAL_*
    Given Snowflake client is logged in
    When A VARCHAR row carrying bare integer values for each interval field is fetched
    Then SQL_C_INTERVAL_YEAR reads year = 5
    And SQL_C_INTERVAL_MONTH reads month = 10
    And SQL_C_INTERVAL_DAY reads day = 15
    And SQL_C_INTERVAL_HOUR reads hour = 8
    And SQL_C_INTERVAL_MINUTE reads minute = 30
    And SQL_C_INTERVAL_SECOND reads second = 45 with fraction = 0

  @odbc_e2e
  Scenario: should preserve negative sign across SQL_C_INTERVAL_* targets
    Given Snowflake client is logged in
    When A VARCHAR row carrying negative interval literals is fetched
    Then SQL_C_INTERVAL_YEAR has interval_sign = SQL_TRUE and year = 5
    And SQL_C_INTERVAL_MONTH has interval_sign = SQL_TRUE and month = 10
    And SQL_C_INTERVAL_DAY has interval_sign = SQL_TRUE and day = 15
    And SQL_C_INTERVAL_YEAR_TO_MONTH has interval_sign = SQL_TRUE, year = 3, month = 6
    And SQL_C_INTERVAL_DAY_TO_HOUR has interval_sign = SQL_TRUE, day = 5, hour = 10

  @odbc_e2e
  Scenario: should fetch zero VARCHAR as SQL_C_INTERVAL_* with unset sign
    # ODBC interval_sign is undefined for zero magnitudes; the driver clears
    # the sign bit when every populated field is zero, regardless of whether
    # the literal carried a "-" prefix.
    Given Snowflake client is logged in
    When A VARCHAR row carrying zero interval values is fetched
    Then SQL_C_INTERVAL_YEAR has year = 0 and interval_sign = SQL_FALSE
    And SQL_C_INTERVAL_YEAR_TO_MONTH has both fields zero and interval_sign = SQL_FALSE
    And '-0' fetched as YEAR keeps interval_sign = SQL_FALSE (zero magnitude has no sign)
    And '-0-0' fetched as YEAR_TO_MONTH keeps interval_sign = SQL_FALSE
    And '0 00:00:00' fetched as DAY_TO_SECOND has all fields zero

  @odbc_e2e
  Scenario: should fetch VARCHAR as composite SQL_C_INTERVAL_YEAR_TO_MONTH
    Given Snowflake client is logged in
    When A VARCHAR row carrying year-month interval literals is fetched
    Then '3-6' produces year = 3, month = 6
    And '0-11' produces year = 0, month = 11
    And '12-0' produces year = 12, month = 0

  @odbc_e2e
  Scenario: should fetch VARCHAR as composite day-time SQL_C_INTERVAL_*
    Given Snowflake client is logged in
    When A VARCHAR row carrying day-time interval literals is fetched
    Then SQL_C_INTERVAL_DAY_TO_HOUR populates day and hour
    And SQL_C_INTERVAL_DAY_TO_MINUTE populates day, hour, minute
    And SQL_C_INTERVAL_DAY_TO_SECOND populates day, hour, minute, second
    And SQL_C_INTERVAL_HOUR_TO_MINUTE populates hour and minute
    And SQL_C_INTERVAL_HOUR_TO_SECOND populates hour, minute, second
    And SQL_C_INTERVAL_MINUTE_TO_SECOND populates minute and second

  @odbc_e2e
  Scenario: should fetch VARCHAR with fractional seconds as SQL_C_INTERVAL_*
    # Fractions in interval literals are normalized to microseconds (the
    # ODBC SQL_INTERVAL_STRUCT fraction field is unsigned int micros).
    Given Snowflake client is logged in
    When A VARCHAR row carrying fractional-second interval literals is fetched
    Then SQL_C_INTERVAL_SECOND parses '12.500000' as second = 12, fraction = 500000 microseconds
    And SQL_C_INTERVAL_MINUTE_TO_SECOND parses '45:30.125' with fraction = 125000 microseconds
    And SQL_C_INTERVAL_HOUR_TO_SECOND parses '12:30:45.999' with fraction = 999000 microseconds
    And SQL_C_INTERVAL_DAY_TO_SECOND parses '2 08:15:30.500' with fraction = 500000 microseconds

  @odbc_e2e
  Scenario: should trim whitespace in VARCHAR -> SQL_C_INTERVAL_*
    Given Snowflake client is logged in
    When A VARCHAR row carrying interval literals padded with whitespace is fetched
    Then leading/trailing whitespace is ignored and SQL_C_INTERVAL_YEAR parses year = 5
    And SQL_C_INTERVAL_YEAR_TO_MONTH parses year = 3, month = 6

  # ============================================================================
  # OUTCOME #2 - SQL_SUCCESS_WITH_INFO, SQLSTATE 01S07
  # ============================================================================

  @odbc_e2e
  Scenario: should truncate trailing fields with SQLSTATE 01S07
    Given Snowflake client is logged in
    When A VARCHAR row carrying literals wider than the target qualifier is fetched
    Then '3-6' fetched as SQL_C_INTERVAL_YEAR keeps year = 3 and warns 01S07 for the dropped month
    And '5 10:30:45' fetched as SQL_C_INTERVAL_DAY keeps day = 5 and warns 01S07
    And '12:30:45' fetched as SQL_C_INTERVAL_HOUR keeps hour = 12 and warns 01S07

  @odbc_e2e
  Scenario: should truncate trailing fields in compound day-time intervals
    Given Snowflake client is logged in
    When A VARCHAR row carrying broader day-time literals than the target qualifier is fetched
    Then '2 08:15:30' fetched as SQL_C_INTERVAL_DAY_TO_HOUR keeps day = 2, hour = 8 with 01S07
    And '2 08:15:30' fetched as SQL_C_INTERVAL_DAY_TO_MINUTE keeps day, hour, minute with 01S07
    And '12:30:45' fetched as SQL_C_INTERVAL_HOUR_TO_MINUTE keeps hour, minute with 01S07

  @odbc_e2e
  Scenario: should warn 01S07 when fractional digits are dropped
    # Fractional magnitude on an integer-only qualifier (YEAR/MONTH/DAY/HOUR/
    # MINUTE) is reported as trailing-field truncation per Appendix D
    # outcome #2.
    Given Snowflake client is logged in
    When A VARCHAR row carrying fractional literals targeted at integer-only qualifiers is fetched
    Then SQL_C_INTERVAL_YEAR keeps year = 5 and warns 01S07 for the dropped fraction

  @odbc_e2e
  Scenario: should not warn when fractional component is exactly zero
    # Regression coverage for the .0-fraction audit fix: a literal like '5.0'
    # carries a syntactic fraction but the magnitude is zero, so an integer-
    # only qualifier loses no information and must return SQL_SUCCESS rather
    # than SQL_SUCCESS_WITH_INFO + 01S07.
    Given Snowflake client is logged in
    When A VARCHAR row carrying a zero-magnitude fraction is fetched as SQL_C_INTERVAL_YEAR
    Then SQL_C_INTERVAL_YEAR returns year = 5 with no truncation warning

  # ============================================================================
  # OUTCOME #3 - SQL_ERROR, SQLSTATE 22015
  # ============================================================================

  @odbc_e2e
  Scenario: should fail with 22015 when leading-field precision is exceeded
    # The implicit SQL_DESC_DATETIME_INTERVAL_PRECISION is 2, so any
    # leading-field magnitude >= 100 must error with 22015.
    Given Snowflake client is logged in
    When A VARCHAR row carrying leading-field magnitudes wider than precision = 2 is fetched
    Then SQL_C_INTERVAL_YEAR returns SQL_ERROR with SQLSTATE 22015
    And SQL_C_INTERVAL_MONTH returns SQL_ERROR with SQLSTATE 22015
    And SQL_C_INTERVAL_DAY returns SQL_ERROR with SQLSTATE 22015
    And SQL_C_INTERVAL_HOUR returns SQL_ERROR with SQLSTATE 22015
    And SQL_C_INTERVAL_SECOND returns SQL_ERROR with SQLSTATE 22015

  @odbc_e2e
  Scenario: should fail with 22015 when composite leading field exceeds precision
    Given Snowflake client is logged in
    When A VARCHAR row carrying composite literals with overflowed leading fields is fetched
    Then SQL_C_INTERVAL_YEAR_TO_MONTH returns SQL_ERROR with SQLSTATE 22015
    And SQL_C_INTERVAL_DAY_TO_SECOND returns SQL_ERROR with SQLSTATE 22015
    And SQL_C_INTERVAL_HOUR_TO_MINUTE returns SQL_ERROR with SQLSTATE 22015

  @odbc_e2e
  Scenario: should respect SQL_DESC_DATETIME_INTERVAL_PRECISION override on the ARD
    # The default leading precision is 2; setting SQL_DESC_DATETIME_INTERVAL_PRECISION
    # via SQLSetDescField on the ARD must be honoured by the VARCHAR -> SQL_C_INTERVAL_*
    # parser. Exercises both an enlargement (5 admits 99999) and a tightening (1 rejects 10).
    Given Snowflake client is logged in
    When SQL_DESC_DATETIME_INTERVAL_PRECISION is set to 5 on the ARD
    Then Precision 5 admits value 99999 for SQL_C_INTERVAL_YEAR
    And Precision 5 still rejects value 100000 for SQL_C_INTERVAL_YEAR
    And Precision 1 admits value 9 for SQL_C_INTERVAL_HOUR
    And Precision 1 rejects value 10 for SQL_C_INTERVAL_HOUR

  # ============================================================================
  # OUTCOME #4 - SQL_ERROR, SQLSTATE 22018
  # ============================================================================

  @odbc_e2e
  Scenario: should reject malformed VARCHAR with SQLSTATE 22018
    Given Snowflake client is logged in
    When A VARCHAR row carrying inputs that aren't valid interval literals is fetched
    Then every interval target returns SQL_ERROR with SQLSTATE 22018

  @odbc_e2e
  Scenario: should reject malformed year-month VARCHAR with SQLSTATE 22018
    Given Snowflake client is logged in
    When A VARCHAR row carrying malformed year-month literals is fetched
    Then every malformed year-month literal returns SQL_ERROR with SQLSTATE 22018

  @odbc_e2e
  Scenario: should reject malformed day-time VARCHAR with SQLSTATE 22018
    Given Snowflake client is logged in
    When A VARCHAR row carrying malformed day-time literals is fetched
    Then every malformed day-time literal returns SQL_ERROR with SQLSTATE 22018

  @odbc_e2e
  Scenario: should reject bare integer for every composite SQL_C_INTERVAL_* target
    # Bare-numeric literals expand into all single-field targets via the
    # parser's "single int" shortcut. Every composite target must reject
    # such an input with 22018 to enforce qualifier shape.
    Given Snowflake client is logged in
    When A VARCHAR carrying a bare integer is fetched as each composite target
    Then every composite target returns SQL_ERROR with SQLSTATE 22018

  @odbc_e2e
  Scenario: should reject out-of-range field magnitudes with SQLSTATE 22018
    # Per ODBC Appendix D outcome #4, a trailing-field magnitude outside its
    # canonical ANSI SQL range is "not a valid interval value" and surfaces
    # SQL_ERROR with SQLSTATE 22018. Enforced ranges:
    #   YEAR_TO_MONTH   trailing MONTH  : 0..=11
    #   *_TO_HOUR       trailing HOUR   : 0..=23
    #   *_TO_MINUTE     trailing MINUTE : 0..=59
    #   *_TO_SECOND     trailing SECOND : 0..=59
    # The leading slot of each composite is precision-driven (22015) and is
    # intentionally NOT range-checked.
    Given Snowflake client is logged in
    When A VARCHAR row carrying out-of-canonical-range trailing fields is fetched
    Then SQL_C_INTERVAL_HOUR_TO_MINUTE rejects minute=61 with SQLSTATE 22018
    And SQL_C_INTERVAL_MINUTE_TO_SECOND rejects second=61 with SQLSTATE 22018
    And SQL_C_INTERVAL_DAY_TO_SECOND rejects hour=24 with SQLSTATE 22018
    And SQL_C_INTERVAL_YEAR_TO_MONTH rejects month=12 with SQLSTATE 22018

  @odbc_e2e
  Scenario: should accept boundary field magnitudes
    # The ANSI ceiling enforced by the driver is exclusive: 24/60/12 reject
    # while the inclusive max of 23/59/11 round-trips cleanly. Pins the
    # boundary behavior for every range-checked trailing slot.
    Given Snowflake client is logged in
    When A VARCHAR row carrying inclusive-max trailing fields is fetched
    Then SQL_C_INTERVAL_HOUR_TO_MINUTE accepts hour=23, minute=59
    And SQL_C_INTERVAL_HOUR_TO_SECOND accepts hour=23, minute=59, second=59
    And SQL_C_INTERVAL_MINUTE_TO_SECOND accepts minute=45, second=59
    And SQL_C_INTERVAL_YEAR_TO_MONTH accepts year=3, month=11

  # ============================================================================
  # NULL HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should return SQL_NULL_DATA when VARCHAR is NULL
    Given Snowflake client is logged in
    When A NULL VARCHAR is fetched as SQL_C_INTERVAL_YEAR
    Then the call returns SQL_SUCCESS with indicator = SQL_NULL_DATA

  # ============================================================================
  # SQLBindCol PATH
  # ============================================================================

  @odbc_e2e
  Scenario: should fetch VARCHAR as SQL_C_INTERVAL_YEAR via SQLBindCol
    Given Snowflake client is logged in
    When SQLBindCol binds a SQL_INTERVAL_STRUCT to the result of a VARCHAR query
    Then the bound struct holds year = 5 with indicator = sizeof(SQL_INTERVAL_STRUCT)

  @odbc_e2e
  Scenario: should reject malformed VARCHAR via SQLBindCol with SQLSTATE 22018
    Given Snowflake client is logged in
    When SQLBindCol binds an interval struct against a malformed VARCHAR
    Then SQLFetch returns SQL_ERROR with a SQLSTATE 22018 diagnostic record
