@odbc
Feature: ODBC DECFLOAT to SQL_C_CHAR/WCHAR conversions

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - DECFLOAT to SQL_C_CHAR
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to SQL_C_CHAR
    Given Snowflake client is logged in
    When DECFLOAT values are fetched as SQL_C_CHAR
    Then SQL_C_CHAR returns correct string representations

  @odbc_e2e
  Scenario: DECFLOAT full precision to SQL_C_CHAR
    Given Snowflake client is logged in
    When A 38-digit DECFLOAT value is fetched as SQL_C_CHAR
    Then SQL_C_CHAR preserves full 38-digit precision

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - DECFLOAT to SQL_C_WCHAR
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT to SQL_C_WCHAR
    Given Snowflake client is logged in
    When DECFLOAT values are fetched as SQL_C_WCHAR
    Then SQL_C_WCHAR returns correct wide string representations

  # ============================================================================
  # TRUNCATION
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT SQL_C_CHAR truncation with small buffer
    Given Snowflake client is logged in
    When A 38-digit DECFLOAT value is fetched into a buffer too small
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  @odbc_e2e
  Scenario: DECFLOAT SQL_C_CHAR exact-fit buffer
    Given Snowflake client is logged in
    When A DECFLOAT value is fetched into a buffer that exactly fits
    Then SQL_SUCCESS is returned with correct content

  @odbc_e2e
  Scenario: DECFLOAT SQL_C_WCHAR truncation with small buffer
    Given Snowflake client is logged in
    When A 38-digit DECFLOAT value is fetched as SQL_C_WCHAR into a buffer too small
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004

  # ============================================================================
  # SQLBindCol
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT using SQLBindCol for SQL_C_CHAR
    Given Snowflake client is logged in
    When DECFLOAT values are fetched using SQLBindCol for SQL_C_CHAR
    Then Bound buffers contain correct DECFLOAT string values

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: DECFLOAT NULL to SQL_C_CHAR types
    Given Snowflake client is logged in
    When A NULL DECFLOAT value is queried
    Then Indicator returns SQL_NULL_DATA for SQL_C_CHAR and SQL_C_WCHAR
