@odbc
Feature: ODBC fixed-point to SQL_C_NUMERIC conversions

  @odbc_e2e
  Scenario: SQL_DECIMAL to SQL_C_NUMERIC
    Given A Snowflake connection is established
    When NUMBER/DECIMAL values are fetched as SQL_C_NUMERIC
    Then SQL_NUMERIC_STRUCT fields match expected sign, precision, scale, and val

  @odbc_e2e
  Scenario: SQL_DECIMAL to SQL_C_NUMERIC with SQL_DESC_PRECISION and SQL_DESC_SCALE
    Given A Snowflake connection is established
    When SQL_DESC_PRECISION and SQL_DESC_SCALE are set via SQLSetDescField
    Then SQL_NUMERIC_STRUCT respects the custom precision and scale settings

  @odbc_e2e
  Scenario: NUMBER NULL to SQL_C_NUMERIC
    Given A Snowflake connection is established
    When A NULL NUMBER value is queried
    Then Indicator returns SQL_NULL_DATA
