@odbc
Feature: ODBC FLOAT to interval type conversions
  # Tests converting Snowflake FLOAT/DOUBLE/REAL type to interval ODBC C types:
  # SQL_C_INTERVAL_YEAR, SQL_C_INTERVAL_MONTH, SQL_C_INTERVAL_DAY,
  # SQL_C_INTERVAL_HOUR, SQL_C_INTERVAL_MINUTE, SQL_C_INTERVAL_SECOND
  # A single FLOAT value maps to the single leading field of the target
  # interval type. Multi-field interval targets are always rejected (22015).

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Single-component interval types
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT to single-field interval types
    Given Snowflake client is logged in
    When Positive, negative, and zero FLOAT values are fetched as interval types
    Then Each single-field interval type returns the correct value and sign

  # ============================================================================
  # TRUNCATION WITH INFO - Fractional truncation (SQLSTATE 01S07)
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT fractional truncation to interval types
    Given Snowflake client is logged in
    When Fractional FLOAT values are fetched as non-second interval types
    Then The fractional part is truncated and SQLSTATE 01S07 is returned

  # ============================================================================
  # SUB-MICROSECOND TRUNCATION (SQLSTATE 01S07)
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT sub-microsecond truncation to interval second
    Given Snowflake client is logged in
    When FLOAT values with sub-microsecond precision are fetched as SQL_C_INTERVAL_SECOND
    Then Sub-microsecond digits are truncated and SQLSTATE 01S07 is returned

  # ============================================================================
  # EDGE CASES - No negative zero
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT to interval - no negative zero
    Given Snowflake client is logged in
    When Negative fractional FLOAT values truncate to zero for non-second intervals
    Then Interval sign is positive when the integer part truncates to zero

  # ============================================================================
  # LEADING FIELD PRECISION - Default precision (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT to interval - default precision rejects values >= 100
    Given Snowflake client is logged in
    When FLOAT values at and beyond the default 2-digit precision are fetched as intervals
    Then Value 99 succeeds and value 100 fails with SQLSTATE 22015

  # ============================================================================
  # LEADING FIELD PRECISION - Custom precision via SQLSetDescField
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT to interval - custom precision via SQLSetDescField
    Given Snowflake client is logged in
    When SQL_DESC_DATETIME_INTERVAL_PRECISION is set on the ARD
    Then Values within custom precision succeed and values beyond it fail

  # ============================================================================
  # ILLEGAL CONVERSIONS - Multi-field interval types (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT to multi-field interval returns 22015
    Given Snowflake client is logged in
    When A FLOAT value is fetched as multi-field interval types
    Then All multi-field interval conversions fail with SQLSTATE 22015

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT NULL to interval C types
    Given Snowflake client is logged in
    When A NULL FLOAT value is queried
    Then Indicator returns SQL_NULL_DATA for all single-field interval types

  # ============================================================================
  # NaN/Infinity edge cases
  # ============================================================================

  @odbc_e2e
  Scenario: FLOAT NaN to interval returns error
    Given Snowflake client is logged in
    When A NaN FLOAT value is fetched as interval types
    Then NaN conversion fails with numeric out of range error

  @odbc_e2e
  Scenario: FLOAT Infinity to interval returns error
    Given Snowflake client is logged in
    When Positive and negative Infinity FLOAT values are fetched as interval types
    Then Infinity conversion fails with numeric out of range error
    And Negative infinity also fails
