@odbc
Feature: REAL conversion to SQL_C_CHAR and SQL_C_WCHAR
  Tests for character string conversions from FLOAT/DOUBLE/REAL columns.

  @odbc_e2e
  Scenario: REAL explicit SQL_C_CHAR
    Given A Snowflake connection
    When FLOAT values are fetched as SQL_C_CHAR
    Then The string representations match the expected numeric values

  @odbc_e2e
  Scenario: REAL to SQL_C_WCHAR
    Given A Snowflake connection
    When FLOAT values are fetched as SQL_C_WCHAR
    Then The wide string representations are non-empty
    And SQL_C_WCHAR matches SQL_C_CHAR for the same value

  @odbc_e2e
  Scenario: REAL SQL_C_CHAR buffer handling
    Given A Snowflake connection
    When FLOAT values are fetched into various buffer sizes as SQL_C_CHAR
    Then The driver returns appropriate SQLSTATE codes for buffer overflow, truncation, and exact fit

  @odbc_e2e
  Scenario: REAL SQL_C_WCHAR buffer handling
    Given A Snowflake connection
    When FLOAT values are fetched into various buffer sizes as SQL_C_WCHAR
    Then The driver returns appropriate SQLSTATE codes for buffer overflow, truncation, and exact fit

  @odbc_e2e
  Scenario: REAL explicit SQL_C_CHAR for special values
    Given A Snowflake connection
    When Special FLOAT values (integer-valued, negative, very small, very large) are fetched as SQL_C_CHAR
    Then The string representations correctly represent each value

  @odbc_e2e
  Scenario: REAL NaN to CHAR produces NaN string
    Given A Snowflake connection
    When NaN is fetched as SQL_C_CHAR
    Then The result is the string "NaN"

  @odbc_e2e
  Scenario: REAL NULL to SQL_C_CHAR
    Given A Snowflake connection
    When A NULL FLOAT value is queried
    Then NULL FLOAT values return SQL_NULL_DATA
