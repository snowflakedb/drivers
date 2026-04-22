@odbc
Feature: ODBC C float types to SQL fixed-point conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When A double value is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When A float value is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE with fraction to SQL_INTEGER truncates
    Given Snowflake client is logged in
    When A fractional double is bound to INTEGER and inserted
    Then The stored value should truncate toward zero when read as integer

  @odbc_e2e
  Scenario: should bind SQL_C_FLOAT zero to SQL_INTEGER
    Given Snowflake client is logged in
    When Float zero is bound and inserted
    Then The value should be read back as integer zero

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE negative to SQL_BIGINT
    Given Snowflake client is logged in
    When A negative double is bound to BIGINT and inserted
    Then The value should be read back correctly as 64-bit integer

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE to SQL_DECIMAL and read back
    Given Snowflake client is logged in
    When A double is bound to DECIMAL(10,2) and inserted
    Then The value should match as a character string

  @odbc_e2e
  Scenario: should reject SQL_C_DOUBLE overflow into NUMBER(3,0)
    Given Snowflake client is logged in
    When A double value exceeding the column precision is bound and inserted
    Then the server rejects the value with SQLSTATE 22003

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE with NULL indicator to SQL_INTEGER
    Given Snowflake client is logged in
    When NULL is bound via SQL_NULL_DATA and inserted
    Then The column should fetch as NULL
