@odbc
Feature: ODBC number incompatible C type conversions
  # Tests that converting Snowflake NUMBER/DECIMAL/NUMERIC SQL type to C types
  # not listed in the ODBC spec conversion table returns the appropriate error.
  # Per the ODBC spec (Appendix D, "SQL to C: Numeric"), exact numeric types
  # (SQL_DECIMAL, SQL_NUMERIC) cannot be converted to temporal or GUID C types.
  # Expected SQLSTATE: 07006 (Restricted data type attribute violation)

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Number to Temporal C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting number to temporal C types
    # ODBC spec does not list SQL_C_TYPE_DATE, SQL_C_TYPE_TIME,
    # SQL_C_TYPE_TIMESTAMP as valid targets for exact numeric SQL types.
    # Expected SQLSTATE: 07006 (Restricted data type attribute violation)
    Given Snowflake client is logged in
    When Query "SELECT 42::NUMBER(10,0)" is executed
    Then SQL_C_TYPE_DATE conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIME conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIMESTAMP conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Number to GUID C Type
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting number to SQL_C_GUID
    # ODBC spec does not list SQL_C_GUID as a valid target for numeric SQL types.
    # Expected SQLSTATE: 07006 (Restricted data type attribute violation)
    # On Windows the Driver Manager may intercept SQL_C_GUID and return HYC00.
    Given Snowflake client is logged in
    When Query "SELECT 42::NUMBER(10,0)" is executed
    Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
