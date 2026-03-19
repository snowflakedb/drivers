@odbc
Feature: ODBC fixed-point to floating-point C type conversions

  @odbc_e2e
  Scenario: Test decimal to floating point conversion
    Given A Snowflake connection is established
    When A table with DECIMAL/NUMBER/INT columns containing value 123 is queried
    Then SQL_C_FLOAT and SQL_C_DOUBLE return correct values without truncation

  @odbc_e2e
  Scenario: SQL_DECIMAL explicit floating point conversions preserve fraction
    Given A Snowflake connection is established
    When A fractional DECIMAL value 123.789 is fetched as float and double
    Then SQL_C_FLOAT and SQL_C_DOUBLE preserve the fractional part

  @odbc_e2e
  Scenario: DECIMAL to floating point precision
    Given A Snowflake connection is established
    When NUMBER values with varying significant digits are fetched as float and double
    Then Precision is preserved within the limits of each floating point type

  @odbc_e2e
  Scenario: DECIMAL multiple rows as SQL_C_DOUBLE
    Given A Snowflake connection is established
    When A table with various DECIMAL(10,3) values is queried
    Then Each row returns the correct double value

  @odbc_e2e
  Scenario: NUMBER NULL to SQL_C_FLOAT and SQL_C_DOUBLE
    Given A Snowflake connection is established
    When A NULL NUMBER value is queried
    Then Indicator returns SQL_NULL_DATA
