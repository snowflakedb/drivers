@odbc
Feature: ODBC float to illegal C type conversions
  # Tests that converting Snowflake FLOAT/DOUBLE/REAL SQL type to C types
  # not listed in the ODBC spec conversion table returns the appropriate error.
  # Per the ODBC spec (Appendix D, "SQL to C: Numeric"), approximate numeric
  # types (SQL_REAL, SQL_FLOAT, SQL_DOUBLE) cannot be converted to temporal,
  # interval, or GUID C types.

  # ============================================================================
  # ILLEGAL CONVERSIONS - Float to Temporal C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting float to temporal C types
    # ODBC spec does not list SQL_C_TYPE_DATE, SQL_C_TYPE_TIME,
    # SQL_C_TYPE_TIMESTAMP as valid targets for approximate numeric SQL types.
    # Expected SQLSTATE: 07006 (Restricted data type attribute violation)
    Given Snowflake client is logged in
    When Query "SELECT 42.5::FLOAT" is executed
    Then SQL_C_TYPE_DATE conversion should fail with restricted data type error
    And SQL_C_TYPE_TIME conversion should fail with restricted data type error
    And SQL_C_TYPE_TIMESTAMP conversion should fail with restricted data type error

  # ============================================================================
  # ILLEGAL CONVERSIONS - Float to Single-Component Interval C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting float to single-component interval C types
    # ODBC spec footnote [c]: single-component interval conversions are
    # "not supported for the approximate numeric data types
    # (SQL_REAL, SQL_FLOAT, or SQL_DOUBLE)."
    Given Snowflake client is logged in
    When Query "SELECT 42.5::FLOAT" is executed
    Then SQL_C_INTERVAL_YEAR conversion should fail with restricted data type error
    And SQL_C_INTERVAL_MONTH conversion should fail with restricted data type error
    And SQL_C_INTERVAL_DAY conversion should fail with restricted data type error
    And SQL_C_INTERVAL_HOUR conversion should fail with restricted data type error
    And SQL_C_INTERVAL_MINUTE conversion should fail with restricted data type error
    And SQL_C_INTERVAL_SECOND conversion should fail with restricted data type error

  # ============================================================================
  # ILLEGAL CONVERSIONS - Float to Compound Interval C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting float to compound interval C types
    Given Snowflake client is logged in
    When Query "SELECT 42.5::FLOAT" is executed
    Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with error
    And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with error
    And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with error
    And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with error
    And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with error
    And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with error
    And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with error

  # ============================================================================
  # ILLEGAL CONVERSIONS - Float to GUID C Type
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting float to SQL_C_GUID
    # ODBC spec does not list SQL_C_GUID as a valid target for numeric SQL types.
    # Expected SQLSTATE: 07006, HY003, or HYC00
    Given Snowflake client is logged in
    When Query "SELECT 42.5::FLOAT" is executed
    Then SQL_C_GUID conversion should fail with restricted data type error
