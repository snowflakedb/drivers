@odbc
Feature: ODBC float incompatible C type conversions
  # Tests that converting Snowflake FLOAT/DOUBLE/REAL SQL type to C types
  # not listed in the ODBC spec conversion table returns the appropriate error.
  # Expected SQLSTATE: 07006 (Restricted data type attribute violation)

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Float to Temporal C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting float to temporal C types
    Given Snowflake client is logged in
    When Query "SELECT 42.5::FLOAT" is executed
    Then SQL_C_TYPE_DATE conversion should fail with restricted data type error
    And SQL_C_TYPE_TIME conversion should fail with restricted data type error
    And SQL_C_TYPE_TIMESTAMP conversion should fail with restricted data type error

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Float to GUID C Type
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting float to SQL_C_GUID
    Given Snowflake client is logged in
    When Query "SELECT 42.5::FLOAT" is executed
    Then SQL_C_GUID conversion should fail with restricted data type error (07006, or HYC00 on Windows)
