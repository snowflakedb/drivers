@odbc
Feature: ODBC C integer types to SQL fixed-point conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When An integer value is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT to SQL_BIGINT and read back
    Given Snowflake client is logged in
    When A 64-bit integer is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_SSHORT to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When A 16-bit integer at minimum value is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_UTINYINT to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When An unsigned 8-bit integer is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_ULONG to SQL_BIGINT and read back
    Given Snowflake client is logged in
    When A 32-bit unsigned integer is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_UBIGINT to SQL_BIGINT and read back
    Given Snowflake client is logged in
    When An unsigned 64-bit maximum value is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind negative integer to SQL_INTEGER
    Given Snowflake client is logged in
    When INT_MIN is bound as SQL_C_SLONG and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_STINYINT to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When A signed 8-bit integer at minimum value is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_USHORT to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When An unsigned 16-bit integer at maximum value is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG zero to SQL_INTEGER
    Given Snowflake client is logged in
    When zero is bound as SQL_C_SLONG and inserted
    Then The value should be read back as zero

  @odbc_e2e
  Scenario: should reject SQL_C_SLONG overflow into NUMBER(3,0)
    Given Snowflake client is logged in
    When An integer exceeding the column precision is bound and inserted
    Then the server rejects the value with an error

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG with NULL indicator
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
