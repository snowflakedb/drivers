@odbc
Feature: ODBC SQL_C_BINARY to SQL temporal types conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY date struct to SQL_TYPE_DATE and read back
    Given Snowflake client is logged in
    When A 6-byte binary buffer containing a SQL_DATE_STRUCT is bound as SQL_TYPE_DATE
    Then The date should be read back correctly

  @odbc_e2e
  Scenario: should reject SQL_C_BINARY with wrong size for SQL_TYPE_DATE
    Given Snowflake client is logged in
    When A 4-byte buffer is bound as SQL_TYPE_DATE
    Then The execution should fail with SQLSTATE 22003

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY time struct to SQL_TYPE_TIME and read back
    Given Snowflake client is logged in
    When A 6-byte binary buffer containing a SQL_TIME_STRUCT is bound as SQL_TYPE_TIME
    Then The time should be read back correctly

  @odbc_e2e
  Scenario: should reject SQL_C_BINARY with wrong size for SQL_TYPE_TIME
    Given Snowflake client is logged in
    When A 4-byte buffer is bound as SQL_TYPE_TIME
    Then The execution should fail with SQLSTATE 22003

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY timestamp struct to SQL_TYPE_TIMESTAMP and read back
    Given Snowflake client is logged in
    When A 16-byte binary buffer containing a SQL_TIMESTAMP_STRUCT is bound as SQL_TYPE_TIMESTAMP
    Then The timestamp should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY timestamp with fractional seconds
    Given Snowflake client is logged in
    When A timestamp struct with 500ms fractional part is bound
    Then The timestamp with fraction should be read back correctly

  @odbc_e2e
  Scenario: should reject SQL_C_BINARY with wrong size for SQL_TYPE_TIMESTAMP
    Given Snowflake client is logged in
    When An 8-byte buffer is bound as SQL_TYPE_TIMESTAMP
    Then The execution should fail with SQLSTATE 22003
