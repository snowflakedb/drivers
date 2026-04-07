@odbc
Feature: ODBC BOOLEAN to interval type conversions
  # Tests converting Snowflake BOOLEAN type to interval ODBC C types.
  # TRUE maps to 1 and FALSE maps to 0 in the target interval field.
  # Multi-field interval targets are always rejected (22015).

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Single-field interval types
  # ============================================================================

  @odbc_e2e
  Scenario: BOOLEAN to single-field interval types
    Given Snowflake client is logged in
    When TRUE and FALSE BOOLEAN values are fetched as interval types
    Then Each single-field interval type returns 1 for TRUE and 0 for FALSE

  # ============================================================================
  # ILLEGAL CONVERSIONS - Multi-field interval types (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: BOOLEAN to multi-field interval returns 22015
    Given Snowflake client is logged in
    When A BOOLEAN value is fetched as multi-field interval types
    Then All multi-field interval conversions fail with SQLSTATE 22015

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: BOOLEAN NULL to interval C types
    Given Snowflake client is logged in
    When A NULL BOOLEAN value is queried
    Then Indicator returns SQL_NULL_DATA for all single-field interval types
