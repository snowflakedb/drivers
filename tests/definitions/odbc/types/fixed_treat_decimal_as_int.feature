@odbc
Feature: ODBC TREAT_DECIMAL_AS_INT and TREAT_BIG_NUMBER_AS_STRING settings

  @odbc_e2e
  Scenario: TREAT_DECIMAL_AS_INT SQL_C_DEFAULT resolves to SBIGINT for scale=0
    Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true
    When DECIMAL values with scale=0 are fetched via SQL_C_DEFAULT
    Then SQL_C_DEFAULT resolves to SBIGINT and returns the correct value

  @odbc_e2e
  Scenario: TREAT_DECIMAL_AS_INT does not affect scale > 0
    Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true
    When A DECIMAL(10,2) value is fetched via SQL_C_DEFAULT
    Then SQL_C_DEFAULT still resolves to SQL_C_CHAR for scale > 0

  @odbc_e2e
  Scenario: TREAT_DECIMAL_AS_INT applies to precision > 18 too
    Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true
    When A NUMBER(38,0) value is fetched via SQL_C_DEFAULT
    Then SQL_C_DEFAULT resolves to SBIGINT even for precision > 18

  @odbc_e2e
  Scenario: TREAT_BIG_NUMBER_AS_STRING overrides TREAT_DECIMAL_AS_INT for precision > 18
    Given A Snowflake connection with both TREAT_DECIMAL_AS_INT and TREAT_BIG_NUMBER_AS_STRING
    When NUMBER(38,0) and DECIMAL(10,0) values are fetched via SQL_C_DEFAULT
    Then NUMBER(38,0) resolves to SQL_C_CHAR and DECIMAL(10,0) still resolves to SBIGINT

  @odbc_e2e
  Scenario: TREAT_DECIMAL_AS_INT with table columns
    Given A Snowflake connection with ODBC_TREAT_DECIMAL_AS_INT=true and a table with mixed columns
    When Each column is fetched via SQL_C_DEFAULT
    Then DECIMAL(10,0) and NUMBER(38,0) resolve to SBIGINT while DECIMAL(10,2) resolves to CHAR
