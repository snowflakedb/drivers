@odbc
Feature: ODBC binary incompatible C type conversions
  # Tests that converting Snowflake BINARY SQL type to C types not listed
  # in the ODBC spec conversion table returns the appropriate error.
  # Per the ODBC spec (Appendix D, "SQL to C: Binary"), SQL_BINARY /
  # SQL_VARBINARY / SQL_LONGVARBINARY can only be converted to SQL_C_CHAR,
  # SQL_C_WCHAR, and SQL_C_BINARY (plus SQL_C_DEFAULT which maps to
  # SQL_C_BINARY). All other C types are incompatible.
  # Expected SQLSTATE: 07006 (Restricted data type attribute violation)

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Binary to Integer C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting binary to integer C types
    # ODBC spec does not list any integer C types as valid targets for binary
    # SQL types. Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_BIT conversion should fail with SQLSTATE 07006
    And SQL_C_TINYINT conversion should fail with SQLSTATE 07006
    And SQL_C_STINYINT conversion should fail with SQLSTATE 07006
    And SQL_C_UTINYINT conversion should fail with SQLSTATE 07006
    And SQL_C_SHORT conversion should fail with SQLSTATE 07006
    And SQL_C_SSHORT conversion should fail with SQLSTATE 07006
    And SQL_C_USHORT conversion should fail with SQLSTATE 07006
    And SQL_C_LONG conversion should fail with SQLSTATE 07006
    And SQL_C_SLONG conversion should fail with SQLSTATE 07006
    And SQL_C_ULONG conversion should fail with SQLSTATE 07006
    And SQL_C_SBIGINT conversion should fail with SQLSTATE 07006
    And SQL_C_UBIGINT conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Binary to Floating-Point C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting binary to floating-point C types
    # ODBC spec does not list SQL_C_FLOAT or SQL_C_DOUBLE as valid targets
    # for binary SQL types. Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_FLOAT conversion should fail with SQLSTATE 07006
    And SQL_C_DOUBLE conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Binary to SQL_C_NUMERIC
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting binary to SQL_C_NUMERIC
    # ODBC spec does not list SQL_C_NUMERIC as a valid target for binary
    # SQL types. Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_NUMERIC conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Binary to Temporal C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting binary to temporal C types
    # ODBC spec does not list SQL_C_TYPE_DATE, SQL_C_TYPE_TIME,
    # SQL_C_TYPE_TIMESTAMP as valid targets for binary SQL types.
    # Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_TYPE_DATE conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIME conversion should fail with SQLSTATE 07006
    And SQL_C_TYPE_TIMESTAMP conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Binary to Single-Component Interval C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting binary to single-component interval C types
    # ODBC spec does not list any interval C types as valid targets for
    # binary SQL types. Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_INTERVAL_YEAR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MONTH conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_SECOND conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Binary to Compound Interval C Types
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting binary to compound interval C types
    # ODBC spec does not list any compound interval C types as valid targets
    # for binary SQL types. Expected SQLSTATE: 07006
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with SQLSTATE 07006
    And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with SQLSTATE 07006

  # ============================================================================
  # INCOMPATIBLE CONVERSIONS - Binary to GUID C Type
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting binary to SQL_C_GUID
    # ODBC spec does not list SQL_C_GUID as a valid target for binary SQL types.
    # Expected SQLSTATE: 07006 (Restricted data type attribute violation)
    # On Windows the Driver Manager may intercept SQL_C_GUID and return HYC00.
    Given Snowflake client is logged in
    When Query "SELECT X'48656C6C6F'::BINARY" is executed
    Then SQL_C_GUID conversion should fail with SQLSTATE 07006 (or HYC00 on Windows)
