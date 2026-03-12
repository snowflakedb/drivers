@odbc
Feature: ODBC boolean to floating point and numeric type conversions
  # Tests converting Snowflake BOOLEAN type to floating point/numeric ODBC C types:
  # SQL_C_FLOAT, SQL_C_DOUBLE, SQL_C_NUMERIC

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Boolean to Floating Point Types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to floating point types
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_FLOAT should return 1.0 for TRUE and 0.0 for FALSE
    And SQL_C_DOUBLE should return 1.0 for TRUE and 0.0 for FALSE

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Boolean to SQL_C_NUMERIC
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to SQL_C_NUMERIC
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_NUMERIC should return value 1 for TRUE and 0 for FALSE with sign=1

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL boolean with numeric and binary C types
    Given Snowflake client is logged in
    When Query "SELECT NULL::BOOLEAN" is executed
    Then SQL_C_FLOAT should return SQL_NULL_DATA indicator
    And SQL_C_DOUBLE should return SQL_NULL_DATA indicator
    And SQL_C_NUMERIC should return SQL_NULL_DATA indicator
