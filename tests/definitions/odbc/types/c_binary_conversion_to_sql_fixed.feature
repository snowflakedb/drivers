@odbc
Feature: ODBC SQL_C_BINARY to SQL integer types conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY i32 to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When A 4-byte binary buffer containing an i32 is bound as SQL_INTEGER and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY i64 to SQL_BIGINT and read back
    Given Snowflake client is logged in
    When An 8-byte binary buffer containing an i64 is bound as SQL_BIGINT and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY i16 to SQL_SMALLINT and read back
    Given Snowflake client is logged in
    When A 2-byte binary buffer containing an i16 is bound as SQL_SMALLINT and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY i8 to SQL_TINYINT and read back
    Given Snowflake client is logged in
    When A 1-byte binary buffer containing an i8 is bound as SQL_TINYINT and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY negative i32 to SQL_INTEGER and read back
    Given Snowflake client is logged in
    When A 4-byte binary buffer containing a negative i32 is bound and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should reject SQL_C_BINARY with wrong size for SQL_INTEGER
    Given Snowflake client is logged in
    When A 3-byte binary buffer is bound as SQL_INTEGER
    Then The execution should fail with SQLSTATE 22003
