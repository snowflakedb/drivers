@odbc
Feature: ODBC REAL (FLOAT/DOUBLE) to SQL_C_BIT conversions
  # Tests conversion of Snowflake FLOAT/DOUBLE/REAL SQL types to SQL_C_BIT.
  # Per ODBC spec, SQL_C_BIT: exact 0 or 1 -> SQL_SUCCESS, fractional -> 01S07,
  # value < 0 or >= 2 -> SQL_ERROR 22003.

  @odbc_e2e
  Scenario: REAL explicit SQL_C_BIT - basic
    Given A Snowflake connection is established
    When Various FLOAT values are fetched as SQL_C_BIT
    Then 0 and 1 succeed, fractions truncate with 01S07, out-of-range returns 22003

  @odbc_e2e
  Scenario: REAL SQL_C_BIT spec compliance
    Given A Snowflake connection is established
    When FLOAT values are fetched as SQL_C_BIT per ODBC spec
    Then 0 and 1 succeed, value 2 returns 22003, negative returns 22003, fractions truncate with 01S07

  @odbc_e2e
  Scenario: REAL SQL_C_BIT rejects negative fractions
    Given A Snowflake connection is established
    When Negative fractional FLOAT values are fetched as SQL_C_BIT
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL NaN to BIT returns error
    Given A Snowflake connection is established
    When NaN FLOAT value is fetched as SQL_C_BIT
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL Infinity to BIT returns 22003
    Given A Snowflake connection is established
    When Infinity FLOAT values are fetched as SQL_C_BIT
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: REAL NULL to SQL_C_BIT
    Given A Snowflake connection is established
    When A NULL FLOAT value is queried
    Then NULL FLOAT values return SQL_NULL_DATA
