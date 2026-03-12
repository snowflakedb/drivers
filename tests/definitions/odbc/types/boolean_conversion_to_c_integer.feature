@odbc
Feature: ODBC boolean to integer type conversions
  # Tests converting Snowflake BOOLEAN type to integer ODBC C types:
  # SQL_C_LONG, SQL_C_SLONG, SQL_C_ULONG, SQL_C_SHORT, SQL_C_SSHORT, SQL_C_USHORT,
  # SQL_C_TINYINT, SQL_C_STINYINT, SQL_C_UTINYINT, SQL_C_SBIGINT, SQL_C_UBIGINT

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Boolean to Integer Types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert boolean to signed integer types
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_LONG should return 1 for TRUE and 0 for FALSE
    And SQL_C_SLONG should return 1 for TRUE and 0 for FALSE
    And SQL_C_SHORT should return 1 for TRUE and 0 for FALSE
    And SQL_C_SSHORT should return 1 for TRUE and 0 for FALSE
    And SQL_C_TINYINT should return 1 for TRUE and 0 for FALSE
    And SQL_C_STINYINT should return 1 for TRUE and 0 for FALSE
    And SQL_C_SBIGINT should return 1 for TRUE and 0 for FALSE

  @odbc_e2e
  Scenario: should convert boolean to unsigned integer types
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then SQL_C_ULONG should return 1 for TRUE and 0 for FALSE
    And SQL_C_USHORT should return 1 for TRUE and 0 for FALSE
    And SQL_C_UTINYINT should return 1 for TRUE and 0 for FALSE
    And SQL_C_UBIGINT should return 1 for TRUE and 0 for FALSE

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL boolean with integer C types
    Given Snowflake client is logged in
    When Query "SELECT NULL::BOOLEAN" is executed
    Then All integer C type conversions should return SQL_NULL_DATA indicator
