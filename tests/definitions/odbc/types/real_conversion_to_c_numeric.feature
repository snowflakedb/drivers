@odbc
Feature: ODBC REAL to SQL_C_NUMERIC conversions

  @odbc_e2e
  Scenario: REAL to SQL_C_NUMERIC
    Given A Snowflake connection is established
    When REAL values are fetched as SQL_C_NUMERIC
    Then SQL_NUMERIC_STRUCT fields match expected sign, val bytes, etc.

  @odbc_e2e
  Scenario: REAL SQL_C_NUMERIC negative zero
    Given A Snowflake connection is established
    When Negative fractional REAL values that truncate to zero are fetched as SQL_C_NUMERIC
    Then SQL_NUMERIC_STRUCT has sign=0 and val=0 (negative zero)

  @odbc_e2e
  Scenario: REAL NaN to NUMERIC returns error
    Given A Snowflake connection is established
    When NaN is fetched as SQL_C_NUMERIC
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL Infinity to NUMERIC returns 22003
    Given A Snowflake connection is established
    When Infinity is fetched as SQL_C_NUMERIC
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL NULL to SQL_C_NUMERIC
    Given A Snowflake connection is established
    When A NULL FLOAT value is queried
    Then NULL FLOAT values return SQL_NULL_DATA
