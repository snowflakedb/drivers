@odbc
Feature: ODBC DECFLOAT to SQL_C_BINARY conversions

  # ============================================================================
  # BASIC BINARY CONVERSIONS
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to SQL_C_BINARY
    Given Snowflake client is logged in
    When A DECFLOAT integer value is fetched as SQL_C_BINARY
    Then SQL_C_BINARY returns SQL_NUMERIC_STRUCT bytes with correct sign and value

  @odbc_e2e
  Scenario: DECFLOAT fractional to SQL_C_BINARY
    Given Snowflake client is logged in
    When A fractional DECFLOAT value is fetched as SQL_C_BINARY
    Then SQL_C_BINARY returns SQL_NUMERIC_STRUCT with integer part

  @odbc_e2e
  Scenario: DECFLOAT negative to SQL_C_BINARY
    Given Snowflake client is logged in
    When A negative DECFLOAT value is fetched as SQL_C_BINARY
    Then SQL_NUMERIC_STRUCT has sign=0 for negative and correct magnitude

  @odbc_e2e
  Scenario: DECFLOAT zero to SQL_C_BINARY
    Given Snowflake client is logged in
    When Zero DECFLOAT value is fetched as SQL_C_BINARY
    Then SQL_NUMERIC_STRUCT has sign=1 and all val bytes zero

  # ============================================================================
  # BUFFER TOO SMALL
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT SQL_C_BINARY buffer too small returns 22003
    Given Snowflake client is logged in
    When A DECFLOAT value is fetched as SQL_C_BINARY into a buffer smaller than SQL_NUMERIC_STRUCT
    Then SQL_ERROR is returned with SQLSTATE 22003

  # ============================================================================
  # OVERFLOW (extreme exponent)
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT extreme exponent to SQL_C_BINARY returns 22003
    Given Snowflake client is logged in
    When A DECFLOAT value with exponent exceeding i128 range is fetched as SQL_C_BINARY
    Then SQL_ERROR is returned with SQLSTATE 22003

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT NULL to SQL_C_BINARY
    Given Snowflake client is logged in
    When A NULL DECFLOAT value is queried
    Then Indicator returns SQL_NULL_DATA
