@odbc
Feature: REAL conversion to SQL_C_DEFAULT
  Tests for SQL_C_DEFAULT conversions from FLOAT/DOUBLE/REAL columns.
  SQL_C_DEFAULT for SQL_DOUBLE resolves to SQL_C_DOUBLE.

  @odbc_e2e
  Scenario: REAL default conversion - basic values
    Given A Snowflake connection
    When FLOAT/DOUBLE values are inserted and fetched via SQL_C_DEFAULT
    Then The correct double values are returned

  @odbc_e2e
  Scenario: REAL default conversion - integer values stored as float
    Given A Snowflake connection
    When Integer values stored as FLOAT are fetched via SQL_C_DEFAULT
    Then The correct double values are returned

  @odbc_e2e
  Scenario: REAL default conversion - extreme values near DBL_MAX
    Given A Snowflake connection
    When Extreme values near DBL_MAX are inserted and fetched via SQL_C_DEFAULT
    Then The correct extreme double values are returned

  @odbc_e2e
  Scenario: REAL default conversion - very small values
    Given A Snowflake connection
    When Very small DOUBLE values are fetched via SQL_C_DEFAULT
    Then The correct small double values are returned

  @odbc_e2e
  Scenario: REAL default conversion - FLOAT, DOUBLE, REAL synonyms produce same result
    Given A Snowflake connection
    When Same value is stored in FLOAT, DOUBLE, REAL columns and fetched via SQL_C_DEFAULT
    Then All three produce the same double value

  @odbc_e2e
  Scenario: REAL SQL_C_DEFAULT matches explicit SQL_C_DOUBLE
    Given A Snowflake connection
    When Values are fetched with SQL_C_DOUBLE and SQL_C_DEFAULT
    Then Results match exactly

  @odbc_e2e
  Scenario: REAL default conversion - multiple rows
    Given A Snowflake connection
    When Multiple DOUBLE rows are fetched via SQL_C_DEFAULT
    Then Each row returns the correct double value

  @odbc_e2e
  Scenario: REAL default conversion - fractional values
    Given A Snowflake connection
    When Fractional FLOAT values are fetched via SQL_C_DEFAULT
    Then The correct fractional double values are returned

  @odbc_e2e
  Scenario: REAL zero is exactly zero
    Given A Snowflake connection
    When Zero FLOAT value is fetched via SQL_C_DEFAULT
    Then The value is exactly zero

  @odbc_e2e
  Scenario: REAL table column conversions
    Given A Snowflake connection
    When A table with FLOAT, DOUBLE, REAL columns is queried
    Then SQL_C_DOUBLE from all column types returns correct values
    And SQL_C_LONG truncates fractional with 01S07
    And SQL_C_CHAR returns string representation

  @odbc_e2e
  Scenario: REAL NULL to SQL_C_DEFAULT
    Given A Snowflake connection
    When A NULL FLOAT value is queried
    Then NULL FLOAT values return SQL_NULL_DATA
