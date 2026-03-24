@odbc
Feature: ODBC REAL to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: REAL to SQL_C_BINARY
    Given A Snowflake connection is established
    When REAL values are fetched as SQL_C_BINARY
    Then The result is a SQL_NUMERIC_STRUCT with correct sign, scale, and val bytes

  @odbc_e2e
  Scenario: REAL SQL_C_BINARY buffer too small returns 22003
    Given A Snowflake connection is established
    When A REAL value is fetched as SQL_C_BINARY into a buffer smaller than SQL_NUMERIC_STRUCT
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL SQL_C_BINARY negative zero
    Given A Snowflake connection is established
    When -0.5 is fetched as SQL_C_BINARY
    Then SQL_SUCCESS_WITH_INFO with 01S07 and SQL_NUMERIC_STRUCT has sign=0, val=0

  @odbc_e2e
  Scenario: REAL NULL to SQL_C_BINARY
    Given A Snowflake connection is established
    When A NULL FLOAT value is queried
    Then NULL FLOAT values return SQL_NULL_DATA
