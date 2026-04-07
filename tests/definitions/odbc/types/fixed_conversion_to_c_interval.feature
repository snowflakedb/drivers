@odbc
Feature: ODBC number to interval type conversions
  # Tests converting Snowflake NUMBER/DECIMAL type to interval ODBC C types:
  # SQL_C_INTERVAL_YEAR, SQL_C_INTERVAL_MONTH, SQL_C_INTERVAL_DAY,
  # SQL_C_INTERVAL_HOUR, SQL_C_INTERVAL_MINUTE, SQL_C_INTERVAL_SECOND
  # A single NUMBER value maps to the single leading field of the target
  # interval type. Multi-field interval targets are always rejected (22015).

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Single-component interval types
  # ============================================================================

  @odbc_e2e
  Scenario: NUMBER to single-field interval types
    Given Snowflake client is logged in
    When Positive, negative, and zero NUMBER values are fetched as SQL_C_INTERVAL_YEAR
    Then SQL_C_INTERVAL_YEAR should contain the correct year values with proper signs
    And SQL_C_INTERVAL_MONTH should contain the correct month value
    And SQL_C_INTERVAL_DAY should contain the correct day value
    And SQL_C_INTERVAL_HOUR should contain the correct hour value
    And SQL_C_INTERVAL_MINUTE should contain the correct minute value
    And SQL_C_INTERVAL_SECOND should contain the correct second values including fractional parts

  # ============================================================================
  # TRUNCATION WITH INFO - Fractional truncation (SQLSTATE 01S07)
  # ============================================================================

  @odbc_e2e
  Scenario: NUMBER to interval - fractional truncation returns 01S07
    # When a NUMBER with a fractional part is converted to an interval type
    # that does not carry fractions (YEAR, MONTH, DAY, HOUR, MINUTE),
    # the fractional part is silently truncated and 01S07 is posted.
    Given Snowflake client is logged in
    When DECIMAL value with fractional part is fetched as SQL_C_INTERVAL_YEAR
    Then SQL_C_INTERVAL_YEAR should truncate the fraction and return 01S07
    And SQL_C_INTERVAL_MONTH should truncate the fraction and return 01S07
    And SQL_C_INTERVAL_DAY should truncate the fraction and return 01S07
    And SQL_C_INTERVAL_HOUR should truncate the fraction and return 01S07
    And SQL_C_INTERVAL_MINUTE should truncate the fraction and return 01S07

  @odbc_e2e
  Scenario: NUMBER to interval - sub-microsecond truncation returns 01S07
    # SQL_INTERVAL_STRUCT.fraction holds microseconds (6 digits).
    # DECIMAL values with > 6 fractional digits lose sub-microsecond precision.
    Given Snowflake client is logged in
    When DECIMAL with 9-digit fractional part is fetched as SQL_C_INTERVAL_SECOND
    Then Sub-microsecond digits should be truncated and 01S07 posted
    And Exact microsecond values should succeed without warning

  # ============================================================================
  # EDGE CASES - Negative zero handling
  # ============================================================================

  @odbc_e2e
  Scenario: NUMBER to interval - no negative zero
    # When a negative fractional value truncates to zero, the interval_sign
    # must be SQL_FALSE (positive), not negative zero.
    Given Snowflake client is logged in
    When Negative fractional DECIMAL is fetched as SQL_C_INTERVAL_YEAR
    Then SQL_C_INTERVAL_YEAR should have positive sign when truncated to zero
    And SQL_C_INTERVAL_MONTH should have positive sign when truncated to zero
    And SQL_C_INTERVAL_DAY should have positive sign when truncated to zero
    And SQL_C_INTERVAL_SECOND should keep negative sign when fraction is nonzero

  # ============================================================================
  # ILLEGAL CONVERSIONS - Multi-field interval types (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: NUMBER to multi-field interval returns 22015
    # A scalar NUMBER cannot be decomposed into multiple interval fields.
    # All multi-field interval targets must fail with 22015.
    Given Snowflake client is logged in
    When Query "SELECT 42::NUMBER(10,0)" is executed and fetched as SQL_C_INTERVAL_YEAR_TO_MONTH
    Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with 22015
    And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with 22015
    And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with 22015
    And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with 22015
    And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with 22015
    And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with 22015
    And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with 22015

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: NUMBER to interval - NULL returns SQL_NULL_DATA
    Given Snowflake client is logged in
    When Query "SELECT NULL::NUMBER(10,0)" is executed and fetched as SQL_C_INTERVAL_YEAR
    Then SQL_C_INTERVAL_YEAR should return SQL_NULL_DATA
    And SQL_C_INTERVAL_MONTH should return SQL_NULL_DATA
    And SQL_C_INTERVAL_DAY should return SQL_NULL_DATA
    And SQL_C_INTERVAL_HOUR should return SQL_NULL_DATA
    And SQL_C_INTERVAL_MINUTE should return SQL_NULL_DATA
    And SQL_C_INTERVAL_SECOND should return SQL_NULL_DATA

  @odbc_e2e
  Scenario: NUMBER NULL to SQL_C_INTERVAL types
    Given A Snowflake connection is established
    When A NULL NUMBER value is queried
    Then Indicator returns SQL_NULL_DATA

  # ============================================================================
  # LEADING FIELD PRECISION - Default precision (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: NUMBER to interval - default precision rejects values >= 100
    # Per ODBC spec, the default interval leading precision is 2 digits.
    # Values with 3+ digits in the leading field must fail with 22015.
    # The old driver does not enforce this (BD#18).
    Given Snowflake client is logged in
    When Value 99 is fetched as SQL_C_INTERVAL_YEAR with default precision
    Then Value 99 should succeed for SQL_C_INTERVAL_YEAR with default precision
    And Value 100 should fail with 22015 for SQL_C_INTERVAL_YEAR
    And Value -100 should fail with 22015 for SQL_C_INTERVAL_DAY
    And Value 100 should fail with 22015 for SQL_C_INTERVAL_SECOND

  # ============================================================================
  # LEADING FIELD PRECISION - Custom precision via SQLSetDescField
  # ============================================================================

  @odbc_e2e
  Scenario: NUMBER to interval - custom precision via SQLSetDescField
    # Applications can increase or decrease the leading field precision
    # by calling SQLSetDescField with SQL_DESC_DATETIME_INTERVAL_PRECISION
    # on the ARD before fetching.
    # The old driver does not support SQL_DESC_DATETIME_INTERVAL_PRECISION (BD#18).
    Given Snowflake client is logged in
    When SQL_DESC_DATETIME_INTERVAL_PRECISION is set to 5 on the ARD
    Then Precision 5 should allow value 99999 for SQL_C_INTERVAL_YEAR
    And Precision 5 should reject value 100000 for SQL_C_INTERVAL_YEAR
    And Precision 1 should allow value 9 for SQL_C_INTERVAL_HOUR
    And Precision 1 should reject value 10 for SQL_C_INTERVAL_HOUR
    And Precision 9 should allow value 999999999 for SQL_C_INTERVAL_SECOND
