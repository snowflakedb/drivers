@odbc
Feature: ODBC fixed-point to integer C type conversions

  @odbc_e2e
  Scenario: Test decimal to integer conversion
    Given A Snowflake connection is established
    When A table with various DECIMAL/NUMBER/INT columns is queried
    Then All integer C types return 123 (exact or with fractional truncation)

  @odbc_e2e
  Scenario: Test integer at limits
    Given A Snowflake connection is established
    When Max and min values for each integer C type are queried
    Then Each type returns its exact limit values

  @odbc_e2e
  Scenario: SQL_DECIMAL explicit integer conversions truncate
    Given A Snowflake connection is established
    When A fractional DECIMAL value 123.789 is fetched as each integer C type
    Then All integer C types return 123 with 01S07 truncation warning

  @odbc_e2e
  Scenario: SQL_DECIMAL truncation and scale
    Given A Snowflake connection is established
    When NUMBER values with various scales are fetched as SQL_C_LONG
    Then Values truncate toward zero and scale is handled correctly

  @odbc_e2e
  Scenario: NUMBER scale=0 - INT and INTEGER types
    Given A Snowflake connection is established
    When A table with INT/INTEGER/BIGINT/SMALLINT/TINYINT columns is queried
    Then Both integer conversions and check_char_success return correct values

  @odbc_e2e
  Scenario: SQL_DECIMAL fractional truncation returns 01S07
    Given A Snowflake connection is established
    When Fractional DECIMAL values are fetched as integer C types
    Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01S07

  @odbc_e2e
  Scenario: SQL_DECIMAL overflow returns 22003
    Given A Snowflake connection is established
    When Out-of-range NUMBER values are fetched as narrow integer C types
    Then SQL_ERROR is returned with SQLSTATE 22003

  @odbc_e2e
  Scenario: NUMBER NULL to SQL_C_INTEGER types
    Given A Snowflake connection is established
    When A NULL NUMBER value is queried
    Then Indicator returns SQL_NULL_DATA

  @odbc_e2e
  Scenario: NUMBER NULL mixed with non-NULL in multiple rows
    Given A Snowflake connection is established
    When A table with mixed NULL and non-NULL rows is queried
    Then NULLs return SQL_NULL_DATA and non-NULLs return correct values

  @odbc_e2e
  Scenario: SQL_DECIMAL SQLGetData NULL without indicator returns 22002
    Given A Snowflake connection is established
    When A NULL value is fetched without providing an indicator pointer
    Then SQL_ERROR is returned with SQLSTATE 22002
