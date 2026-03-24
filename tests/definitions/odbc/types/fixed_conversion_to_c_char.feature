@odbc
Feature: ODBC fixed-point to SQL_C_CHAR/WCHAR conversions

  @odbc_e2e
  Scenario: Test decimal to SQL_C_CHAR and SQL_C_DEFAULT conversion
    Given A Snowflake connection is established
    When A table with various DECIMAL/NUMBER/INT columns is queried
    Then SQL_C_CHAR and SQL_C_DEFAULT return correct string representations

  @odbc_e2e
  Scenario: SQL_DECIMAL default conversion
    Given A Snowflake connection is established
    When A table with DECIMAL columns is queried
    Then SQL_C_DEFAULT returns matching string values

  @odbc_e2e
  Scenario: SQL_DECIMAL default conversion - large precision
    Given A Snowflake connection is established
    When A table with NUMBER(38) columns is queried
    Then SQL_C_DEFAULT preserves full 38-digit precision

  @odbc_e2e
  Scenario: SQL_DECIMAL to SQL_C_WCHAR
    Given A Snowflake connection is established
    When Various NUMBER values are queried as SQL_C_WCHAR
    Then SQL_C_WCHAR returns matching wide strings

  @odbc_e2e
  Scenario: SQL_DECIMAL SQL_C_CHAR buffer handling
    Given A Snowflake connection is established
    When NUMBER values are fetched into various buffer sizes
    Then The driver returns appropriate SQLSTATE codes for buffer overflow, truncation, and exact fit

  @odbc_e2e
  Scenario: Without TREAT_DECIMAL_AS_INT default is SQL_C_CHAR for scale=0
    Given A Snowflake connection is established
    When An integer DECIMAL value is queried with SQL_C_DEFAULT
    Then SQL_C_DEFAULT resolves to SQL_C_CHAR and returns string "42"

  @odbc_e2e
  Scenario: Test string at limits
    Given A Snowflake connection is established
    When Max and min 37-digit NUMBER values are fetched as SQL_C_CHAR and SQL_C_DEFAULT
    Then Both SQL_C_CHAR and SQL_C_DEFAULT return correct extreme values

  @odbc_e2e
  Scenario: DECIMAL multiple rows as SQL_C_CHAR
    Given A Snowflake connection is established
    When A table with various DECIMAL(10,2) values is queried
    Then Each row returns the correct string value

  @odbc_e2e
  Scenario: NUMBER NULL to SQL_C_CHAR types
    Given A Snowflake connection is established
    When A NULL NUMBER value is queried
    Then Indicator returns SQL_NULL_DATA
