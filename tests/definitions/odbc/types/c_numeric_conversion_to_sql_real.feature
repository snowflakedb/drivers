@odbc
Feature: ODBC SQL_C_NUMERIC to floating SQL type conversions

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When SQL_C_NUMERIC is bound to SQL_DOUBLE and inserted into FLOAT
    Then the value is read back as SQL_C_DOUBLE 42

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC with scale to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_NUMERIC with scale 2 (3.14) is bound to SQL_DOUBLE and inserted
    Then the value is approximately 3.14

  @odbc_e2e
  Scenario: should bind large SQL_C_NUMERIC exceeding 64-bit to SQL_DOUBLE
    Given Snowflake client is logged in
    When a large SQL_C_NUMERIC exceeding 64-bit range is bound to SQL_DOUBLE and inserted
    Then the value is approximately 1e20

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC with NULL indicator to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_NUMERIC is bound with SQL_NULL_DATA to SQL_DOUBLE and inserted
    Then the column is NULL when fetched as SQL_C_DOUBLE
