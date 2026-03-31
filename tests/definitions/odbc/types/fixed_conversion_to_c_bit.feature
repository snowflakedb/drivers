@odbc
Feature: ODBC fixed-point to SQL_C_BIT conversions

  @odbc_e2e
  Scenario: SQL_C_BIT spec compliance
    Given A Snowflake connection is established
    When Various NUMBER/DECIMAL values are fetched as SQL_C_BIT
    Then Values 0 and 1 succeed, fractions truncate with 01S07, and out-of-range returns 22003

  @odbc_e2e
  Scenario: NUMBER NULL to SQL_C_BIT
    Given A Snowflake connection is established
    When A NULL NUMBER value is queried
    Then Indicator returns SQL_NULL_DATA
