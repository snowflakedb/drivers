@odbc
Feature: ODBC boolean incompatible C type conversions
  # Tests that converting Snowflake BOOLEAN SQL type to incompatible C types
  # returns the appropriate error.
  # Expected SQLSTATE: 07006 (Restricted data type attribute violation)

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Boolean to Temporal C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting boolean to temporal C types
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN" is executed
    Then SQL_C_TYPE_DATE conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIME conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIMESTAMP conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Boolean to GUID C Type
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting boolean to SQL_C_GUID
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN" is executed
    Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
