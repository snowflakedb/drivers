@odbc
Feature: ODBC boolean incompatible C type conversions
  # Tests that converting Snowflake BOOLEAN SQL type to C types not listed
  # in the ODBC spec conversion table returns the appropriate error.
  # Per the ODBC spec (Appendix D, "SQL to C: Bit"), SQL_BIT can be
  # converted to SQL_C_BIT, SQL_C_CHAR, SQL_C_WCHAR, SQL_C_BINARY, all
  # integer C types, SQL_C_FLOAT, SQL_C_DOUBLE, and SQL_C_NUMERIC (plus
  # SQL_C_DEFAULT which maps to SQL_C_BIT).
  # All other C types are incompatible.
  # Expected SQLSTATE: 07006 (Restricted data type attribute violation)

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Boolean to Temporal C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting boolean to temporal C types
    # ODBC spec does not list SQL_C_TYPE_DATE, SQL_C_TYPE_TIME,
    # SQL_C_TYPE_TIMESTAMP as valid targets for SQL_BIT.
    # Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN" is executed
    Then SQL_C_TYPE_DATE conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIME conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIMESTAMP conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Boolean to Single-Component Interval C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting boolean to single-component interval C types
    # ODBC spec does not list any interval C types as valid targets for
    # SQL_BIT. Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN" is executed
    Then SQL_C_INTERVAL_YEAR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MONTH conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_SECOND conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Boolean to Compound Interval C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting boolean to compound interval C types
    # ODBC spec does not list any compound interval C types as valid targets
    # for SQL_BIT. Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN" is executed
    Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Boolean to GUID C Type
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting boolean to SQL_C_GUID
    # ODBC spec does not list SQL_C_GUID as a valid target for SQL_BIT.
    # Expected SQLSTATE: 07006 (Restricted data type attribute violation)
    # On Windows the Driver Manager may intercept SQL_C_GUID and return HYC00.
    Given Snowflake client is logged in
    When Query "SELECT TRUE::BOOLEAN" is executed
    Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
