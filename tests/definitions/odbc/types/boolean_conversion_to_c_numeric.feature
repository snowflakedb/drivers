@odbc
Feature: ODBC boolean to SQL_C_NUMERIC type conversion
  # Tests converting Snowflake BOOLEAN type to SQL_C_NUMERIC ODBC C type

  # ============================================================================
  # SUCCESSFUL CONVERSIONS
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
  Scenario: should handle NULL boolean with SQL_C_NUMERIC
    Given Snowflake client is logged in
    When Query "SELECT NULL::BOOLEAN" is executed
    Then SQL_C_NUMERIC should return SQL_NULL_DATA indicator
