@odbc
Feature: ODBC numeric C types to SQL_VARCHAR conversions

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_SLONG 42 is bound to SQL_VARCHAR and inserted
    Then fetching as SQL_C_CHAR yields 42

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_DOUBLE 3.14 is bound to SQL_VARCHAR and inserted
    Then the string representation contains 3.14

  @odbc_e2e
  Scenario: should bind SQL_C_BIT to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_BIT 1 is bound to SQL_VARCHAR and inserted
    Then the value is read back as the string 1

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_NUMERIC (123.45) is bound to SQL_VARCHAR and inserted
    Then the value is read back as 123.45

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_SBIGINT 9999999999 is bound to SQL_VARCHAR and inserted
    Then the value is read back as 9999999999

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG with NULL indicator to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_SLONG is bound with SQL_NULL_DATA and inserted
    Then the column is NULL when fetched as SQL_C_CHAR
