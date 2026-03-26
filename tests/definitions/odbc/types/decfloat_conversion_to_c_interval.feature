@odbc
Feature: ODBC DECFLOAT to interval type conversions
  # Tests converting Snowflake DECFLOAT type to interval ODBC C types:
  # SQL_C_INTERVAL_YEAR, SQL_C_INTERVAL_MONTH, SQL_C_INTERVAL_DAY,
  # SQL_C_INTERVAL_HOUR, SQL_C_INTERVAL_MINUTE, SQL_C_INTERVAL_SECOND
  # A single DECFLOAT value maps to the single leading field of the target
  # interval type. Multi-field interval targets are always rejected (22015).

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Single-component interval types
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to single-field interval types
    Given Snowflake client is logged in
    When Positive, negative, and zero DECFLOAT values are fetched as interval types
    Then Each single-field interval type returns the correct value and sign

  # ============================================================================
  # TRUNCATION WITH INFO - Fractional truncation (SQLSTATE 01S07)
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT fractional truncation to interval types
    Given Snowflake client is logged in
    When Fractional DECFLOAT values are fetched as non-second interval types
    Then The fractional part is truncated and SQLSTATE 01S07 is returned

  # ============================================================================
  # SUB-MICROSECOND TRUNCATION (SQLSTATE 01S07)
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT sub-microsecond truncation to interval second
    Given Snowflake client is logged in
    When DECFLOAT values with more than 6 decimal places are fetched as SQL_C_INTERVAL_SECOND
    Then Sub-microsecond digits are truncated and SQLSTATE 01S07 is returned

  # ============================================================================
  # EDGE CASES - No negative zero
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to interval - no negative zero
    Given Snowflake client is logged in
    When Negative fractional DECFLOAT values truncate to zero for non-second intervals
    Then Interval sign is positive when the integer part truncates to zero

  # ============================================================================
  # EDGE CASES - Positive exponent
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT with positive exponent to interval
    Given Snowflake client is logged in
    When DECFLOAT values with positive exponents are fetched as interval types
    Then The exponent is applied correctly to produce the interval value

  # ============================================================================
  # LEADING FIELD PRECISION - Default precision (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to interval - default precision rejects values >= 100
    Given Snowflake client is logged in
    When DECFLOAT values at and beyond the default 2-digit precision are fetched as intervals
    Then Value 99 succeeds and value 100 fails with SQLSTATE 22015

  # ============================================================================
  # LEADING FIELD PRECISION - Custom precision via SQLSetDescField
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to interval - custom precision via SQLSetDescField
    Given Snowflake client is logged in
    When SQL_DESC_DATETIME_INTERVAL_PRECISION is set on the ARD
    Then Values within custom precision succeed and values beyond it fail

  # ============================================================================
  # ILLEGAL CONVERSIONS - Multi-field interval types (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to multi-field interval returns 22015
    Given Snowflake client is logged in
    When A DECFLOAT value is fetched as multi-field interval types
    Then All multi-field interval conversions fail with SQLSTATE 22015

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT NULL to interval C types
    Given Snowflake client is logged in
    When A NULL DECFLOAT value is queried
    Then Indicator returns SQL_NULL_DATA for all single-field interval types
