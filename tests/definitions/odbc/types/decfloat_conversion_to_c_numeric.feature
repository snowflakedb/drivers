@odbc
Feature: ODBC DECFLOAT to SQL_C_NUMERIC conversions

  # ============================================================================
  # BASIC SQL_C_NUMERIC CONVERSIONS
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to SQL_C_NUMERIC
    Given Snowflake client is logged in
    When DECFLOAT values are fetched as SQL_C_NUMERIC
    Then SQL_NUMERIC_STRUCT fields match expected sign, precision, scale, and val

  # ============================================================================
  # FRACTIONAL TRUNCATION
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT fractional truncation to SQL_C_NUMERIC
    Given Snowflake client is logged in
    When A fractional DECFLOAT value is fetched as SQL_C_NUMERIC with default scale=0
    Then Value is truncated to integer part with SQLSTATE 01S07

  # ============================================================================
  # OVERFLOW (extreme exponent)
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT extreme exponent to SQL_C_NUMERIC returns 22003
    Given Snowflake client is logged in
    When A DECFLOAT value with exponent exceeding u128 range is fetched as SQL_C_NUMERIC
    Then SQL_ERROR is returned with SQLSTATE 22003

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT NULL to SQL_C_NUMERIC
    Given Snowflake client is logged in
    When A NULL DECFLOAT value is queried
    Then Indicator returns SQL_NULL_DATA
