@odbc
Feature: ODBC DECFLOAT to SQL_C_BIT conversions

  # ============================================================================
  # BASIC BIT CONVERSIONS
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to SQL_C_BIT spec compliance
    Given Snowflake client is logged in
    When Various DECFLOAT values are fetched as SQL_C_BIT
    Then 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT NULL to SQL_C_BIT
    Given Snowflake client is logged in
    When A NULL DECFLOAT value is queried
    Then Indicator returns SQL_NULL_DATA
