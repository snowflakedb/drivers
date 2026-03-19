@odbc
Feature: REAL conversion to SQL_C_DOUBLE and SQL_C_FLOAT
  Tests for explicit SQL_C_DOUBLE and SQL_C_FLOAT conversions from FLOAT/DOUBLE/REAL columns.

  @odbc_e2e
  Scenario: REAL explicit SQL_C_DOUBLE
    Given A Snowflake connection
    When FLOAT value is fetched as SQL_C_DOUBLE
    Then The value matches within relative tolerance

  @odbc_e2e
  Scenario: REAL explicit SQL_C_FLOAT
    Given A Snowflake connection
    When FLOAT value is fetched as SQL_C_FLOAT
    Then The value matches within relative tolerance

  @odbc_e2e
  Scenario: REAL precision - Snowflake FLOAT has ~15 significant digits
    Given A Snowflake connection
    When FLOAT value with 15 significant digits is fetched as SQL_C_DOUBLE
    Then The value matches within relative tolerance

  @odbc_e2e
  Scenario: REAL negative zero
    Given A Snowflake connection
    When Negative zero FLOAT is fetched as SQL_C_DOUBLE
    Then The value is exactly zero

  @odbc_e2e
  Scenario: REAL SQL_C_FLOAT precision loss
    Given A Snowflake connection
    When Values representable in f32 are fetched as SQL_C_FLOAT
    Then They match without truncation
    And Large value representable in f32 succeeds

  @odbc_e2e
  Scenario: REAL SQL_C_FLOAT overflow returns 22003
    Given A Snowflake connection
    When Values exceeding f32 range are fetched as SQL_C_FLOAT
    Then Positive overflow returns 22003
    And Negative overflow returns 22003
    And Large value within f32 range succeeds

  @odbc_e2e
  Scenario: REAL NULL to SQL_C_FLOAT and SQL_C_DOUBLE
    Given A Snowflake connection
    When NULL FLOAT values are queried
    Then NULL FLOAT values return SQL_NULL_DATA

  @odbc_e2e
  Scenario: REAL NULL mixed with non-NULL in multiple rows
    Given A Snowflake connection
    When A table with mixed NULL and non-NULL FLOAT rows is queried
    Then NULLs return SQL_NULL_DATA and non-NULLs return correct values

  @odbc_e2e
  Scenario: REAL SQLGetData NULL without indicator returns 22002
    Given A Snowflake connection
    When A NULL FLOAT value is fetched without providing an indicator pointer
    Then SQL_ERROR is returned with SQLSTATE 22002
