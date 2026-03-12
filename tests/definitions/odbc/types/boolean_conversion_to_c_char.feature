@odbc
Feature: ODBC boolean to character type conversions
  # Tests converting Snowflake BOOLEAN type to character ODBC C types:
  # SQL_C_CHAR, SQL_C_WCHAR

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Boolean to SQL_C_CHAR
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to SQL_C_CHAR
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_CHAR should return "1" for TRUE and "0" for FALSE

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Boolean to SQL_C_WCHAR
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to SQL_C_WCHAR
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_WCHAR should return "1" for TRUE and "0" for FALSE

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL boolean with character C types
    Given Snowflake client is logged in
    When Query "SELECT NULL::BOOLEAN" is executed
    Then SQL_C_CHAR should return SQL_NULL_DATA indicator
    And SQL_C_WCHAR should return SQL_NULL_DATA indicator
