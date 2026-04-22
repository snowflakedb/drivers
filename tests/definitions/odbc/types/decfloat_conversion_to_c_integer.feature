@odbc
Feature: ODBC DECFLOAT to integer C type conversions

  # ============================================================================
  # BASIC INTEGER CONVERSIONS
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to integer C types
    Given Snowflake client is logged in
    When A small DECFLOAT integer is fetched as various integer C types
    Then All integer C types return 42

  # ============================================================================
  # FRACTIONAL TRUNCATION
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT fractional truncation to integer C types
    Given Snowflake client is logged in
    When A fractional DECFLOAT value is fetched as integer C types
    Then Integer C types return 123 with SQLSTATE 01S07

  # ============================================================================
  # OVERFLOW
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT overflow to integer C types
    Given Snowflake client is logged in
    When A large DECFLOAT value is fetched as integer C types
    Then SQL_ERROR is returned with SQLSTATE 22003

  # ============================================================================
  # TYPE BOUNDARIES
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT integer type boundaries
    Given Snowflake client is logged in
    When DECFLOAT values at exact type boundaries are fetched as integer C types
    Then Values at exact min/max boundaries are returned correctly

  # ============================================================================
  # PER-TYPE OVERFLOW (22003)
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT per-type integer overflow
    Given Snowflake client is logged in
    When DECFLOAT values just beyond type boundaries are fetched as integer C types
    Then SQL_ERROR is returned with SQLSTATE 22003 for each type

  # ============================================================================
  # FRACTIONAL TRUNCATION - ALL TYPES
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT fractional truncation for all integer types
    Given Snowflake client is logged in
    When A fractional DECFLOAT value is fetched as each integer C type
    Then All integer C types return 123 with SQLSTATE 01S07

  # ============================================================================
  # POSITIVE EXPONENT
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT with positive exponent to integer
    Given Snowflake client is logged in
    When DECFLOAT values with positive exponents are fetched as SQL_C_LONG
    Then The exponent is applied correctly to produce the integer result

  # ============================================================================
  # TINY VALUE TRUNCATION
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT tiny value to integer truncates to zero
    Given Snowflake client is logged in
    When A very small fractional DECFLOAT value is fetched as SQL_C_LONG
    Then Value truncates to 0 with SQLSTATE 01S07

  # ============================================================================
  # SQLBindCol
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT using SQLBindCol for SQL_C_LONG
    Given Snowflake client is logged in
    When DECFLOAT values are fetched using SQLBindCol for SQL_C_LONG
    Then Bound integers contain correct DECFLOAT values

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT NULL to integer C types
    Given Snowflake client is logged in
    When A NULL DECFLOAT value is queried
    Then Indicator returns SQL_NULL_DATA
