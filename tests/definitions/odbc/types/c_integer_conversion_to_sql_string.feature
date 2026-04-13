@odbc
Feature: ODBC C integer types to SQL string conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When An integer value is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When A 64-bit integer is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_SSHORT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When A 16-bit integer at minimum value is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_UTINYINT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When An unsigned 8-bit integer is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_ULONG to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When A 32-bit unsigned integer is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_STINYINT negative to SQL_VARCHAR
    Given Snowflake client is logged in
    When A signed 8-bit integer at minimum value is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_USHORT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When An unsigned 16-bit integer at maximum value is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_UBIGINT max to SQL_VARCHAR
    Given Snowflake client is logged in
    When An unsigned 64-bit maximum value is bound to SQL_VARCHAR and inserted
    Then The value should be read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG zero to SQL_VARCHAR
    Given Snowflake client is logged in
    When zero is bound as SQL_C_SLONG to SQL_VARCHAR and inserted
    Then The value should be read back as zero

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG with NULL indicator to SQL_VARCHAR
    Given Snowflake client is logged in
    When A NULL parameter is bound using SQL_NULL_DATA
    Then The stored value should be NULL
