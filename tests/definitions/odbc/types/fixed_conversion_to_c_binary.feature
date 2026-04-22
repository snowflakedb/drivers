@odbc
Feature: ODBC fixed-point to SQL_C_BINARY conversions

  @odbc_e2e
  Scenario: SQL_DECIMAL to SQL_C_BINARY
    Given A Snowflake connection is established
    When NUMBER values are fetched as SQL_C_BINARY
    Then The result is a SQL_NUMERIC_STRUCT with correct sign, scale, and val bytes

  @odbc_e2e
  Scenario: SQL_DECIMAL SQL_C_BINARY buffer too small returns 22003
    Given A Snowflake connection is established
    When A NUMBER value is fetched as SQL_C_BINARY into a buffer smaller than SQL_NUMERIC_STRUCT
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: NUMBER NULL to SQL_C_BINARY
    Given A Snowflake connection is established
    When A NULL NUMBER value is queried
    Then Indicator returns SQL_NULL_DATA
