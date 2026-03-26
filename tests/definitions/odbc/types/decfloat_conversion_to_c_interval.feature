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
