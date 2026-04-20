@odbc
Feature: ODBC SQL_C_BINARY to SQL real types conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY f64 to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When An 8-byte binary buffer containing an f64 is bound as SQL_DOUBLE and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY f32 to SQL_REAL and read back
    Given Snowflake client is logged in
    When A 4-byte binary buffer containing an f32 is bound as SQL_REAL and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should accept SQL_C_BINARY NaN for SQL_DOUBLE
    Given Snowflake client is logged in
    When An 8-byte binary buffer containing NaN is bound as SQL_DOUBLE
    Then The NaN value should round-trip back to the client as NaN

  @odbc_e2e
  Scenario: should accept SQL_C_BINARY infinity for SQL_REAL
    Given Snowflake client is logged in
    When A 4-byte binary buffer containing infinity is bound as SQL_REAL
    Then The infinity value should round-trip back to the client as +Infinity

  @odbc_e2e
  Scenario: should reject SQL_C_BINARY with wrong size for SQL_DOUBLE
    Given Snowflake client is logged in
    When A 3-byte binary buffer is bound as SQL_DOUBLE
    Then The execution should fail with SQLSTATE 22003
