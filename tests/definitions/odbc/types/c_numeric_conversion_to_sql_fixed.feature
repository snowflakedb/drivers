@odbc
Feature: ODBC SQL_C_NUMERIC to fixed SQL type conversions

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When SQL_C_NUMERIC is bound to SQL_INTEGER and a row is inserted into NUMBER
    Then the stored value is read back as SQL_C_SBIGINT 42

  @odbc_e2e
  Scenario: should bind negative SQL_C_NUMERIC to SQL_BIGINT
    Given Snowflake client is logged in
    When a negative SQL_C_NUMERIC (sign 0, magnitude 99) is bound to SQL_BIGINT and inserted
    Then the value is read back as -99

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC with scale to SQL_DECIMAL
    Given Snowflake client is logged in
    When SQL_C_NUMERIC with scale 2 (123.45) is bound to SQL_DECIMAL and inserted
    Then fetching as SQL_C_CHAR yields 123.45

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC zero
    Given Snowflake client is logged in
    When SQL_C_NUMERIC zero is bound to SQL_INTEGER and inserted
    Then the value is read back as 0

  @odbc_e2e
  Scenario: should bind large SQL_C_NUMERIC exceeding 64-bit range
    Given Snowflake client is logged in
    When a large SQL_C_NUMERIC exceeding 64-bit range is bound and inserted
    Then the value is read back as the string 100000000000000000000

  @odbc_e2e
  Scenario: should reject SQL_C_NUMERIC overflow into NUMBER(3,0)
    Given Snowflake client is logged in
    When SQL_C_NUMERIC with value 99999 is bound to a NUMBER(3,0) column and inserted
    Then the server rejects the value with SQLSTATE 22003

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC with NULL indicator
    Given Snowflake client is logged in
    When SQL_C_NUMERIC is bound with SQL_NULL_DATA and inserted
    Then the column is NULL when fetched
