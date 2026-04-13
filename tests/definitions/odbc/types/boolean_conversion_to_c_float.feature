@odbc
Feature: ODBC boolean to floating point type conversions
  # Tests converting Snowflake BOOLEAN type to floating point ODBC C types:
  # SQL_C_FLOAT, SQL_C_DOUBLE

  # ============================================================================
  # SUCCESSFUL CONVERSIONS
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to c_type
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then <c_type> should return 1.0 for TRUE and 0.0 for FALSE

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL boolean with floating point c_type
    Given Snowflake client is logged in
    When Query "SELECT NULL::BOOLEAN" is executed
    Then <c_type> should return SQL_NULL_DATA indicator
