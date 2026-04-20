@odbc
Feature: ODBC SQL_C_BINARY to SQL decimal/numeric types conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY numeric struct to SQL_DECIMAL and read back
    Given Snowflake client is logged in
    When A 19-byte binary buffer containing a SQL_NUMERIC_STRUCT is bound as SQL_DECIMAL
    Then The value 123.45 should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_BINARY numeric struct integer to SQL_NUMERIC
    Given Snowflake client is logged in
    When A SQL_NUMERIC_STRUCT with scale=0 is bound as SQL_NUMERIC
    Then The integer value should be read back correctly

  @odbc_e2e
  Scenario: should reject SQL_C_BINARY with wrong size for SQL_DECIMAL
    Given Snowflake client is logged in
    When A 10-byte buffer (not 19) is bound as SQL_DECIMAL
    Then The execution should fail with SQLSTATE 22003
