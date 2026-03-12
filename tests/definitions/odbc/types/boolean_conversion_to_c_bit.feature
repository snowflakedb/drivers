@odbc
Feature: ODBC boolean to bit/default type conversions
  # Tests converting Snowflake BOOLEAN type to SQL_C_BIT and SQL_C_DEFAULT

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Boolean to SQL_C_BIT
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to SQL_C_BIT
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_BIT should return 1 for TRUE and 0 for FALSE

  @odbc_e2e
  Scenario: should convert boolean to SQL_C_DEFAULT
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_DEFAULT should return the same values as SQL_C_BIT

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL boolean with SQL_C_BIT
    Given Snowflake client is logged in
    When Query "SELECT NULL::BOOLEAN" is executed
    Then SQL_C_BIT should return SQL_NULL_DATA indicator

  # ============================================================================
  # CONVERSION WITH SQLBindCol
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean using SQLBindCol for SQL_C_BIT
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed with SQLBindCol for SQL_C_BIT
    Then the bound values should be 1 and 0
