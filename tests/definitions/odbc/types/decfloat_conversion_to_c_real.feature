@odbc
Feature: ODBC DECFLOAT to floating-point C type conversions

  # ============================================================================
  # SQL_C_DOUBLE
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to SQL_C_DOUBLE
    Given Snowflake client is logged in
    When DECFLOAT values in float64 range are fetched as SQL_C_DOUBLE
    Then SQL_C_DOUBLE returns approximately correct values

  # ============================================================================
  # SQL_C_FLOAT
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to SQL_C_FLOAT
    Given Snowflake client is logged in
    When DECFLOAT values in float32 range are fetched as SQL_C_FLOAT
    Then SQL_C_FLOAT returns approximately correct values

  # ============================================================================
  # PRECISION LOSS
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT precision loss to SQL_C_DOUBLE
    Given Snowflake client is logged in
    When A 38-digit DECFLOAT value is fetched as SQL_C_DOUBLE
    Then Value is approximately correct but precision beyond float64 is lost

  # ============================================================================
  # OVERFLOW
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT SQL_C_FLOAT overflow
    Given Snowflake client is logged in
    When A DECFLOAT value beyond float32 range but within float64 range is fetched as SQL_C_FLOAT
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: DECFLOAT SQL_C_DOUBLE overflow
    Given Snowflake client is logged in
    When A DECFLOAT value beyond float64 range is fetched as SQL_C_DOUBLE
    Then SQL_ERROR is returned with SQLSTATE 22003

  # ============================================================================
  # SQLBindCol
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT using SQLBindCol for SQL_C_DOUBLE
    Given Snowflake client is logged in
    When DECFLOAT values are fetched using SQLBindCol for SQL_C_DOUBLE
    Then Bound doubles contain correct DECFLOAT values

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT NULL to floating-point C types
    Given Snowflake client is logged in
    When A NULL DECFLOAT value is queried
    Then Indicator returns SQL_NULL_DATA
