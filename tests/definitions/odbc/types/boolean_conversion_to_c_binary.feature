@odbc
Feature: ODBC boolean to binary type conversions
  # Tests converting Snowflake BOOLEAN type to binary ODBC C type:
  # SQL_C_BINARY

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Boolean to SQL_C_BINARY
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to SQL_C_BINARY
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_BINARY should return byte 0x01 for TRUE and 0x00 for FALSE
