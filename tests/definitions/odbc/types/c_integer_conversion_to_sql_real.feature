@odbc
Feature: ODBC C integer types to SQL real conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When An integer value is bound as SQL_C_SLONG and inserted
    Then The value should be read back as double

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When A 64-bit integer is bound as SQL_C_SBIGINT and inserted
    Then The value should be read back as double

  @odbc_e2e
  Scenario: should bind SQL_C_SSHORT to SQL_REAL and read back
    Given Snowflake client is logged in
    When A 16-bit integer is bound as SQL_C_SSHORT and inserted
    Then The value should be read back as double

  @odbc_e2e
  Scenario: should bind SQL_C_UTINYINT to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When An unsigned 8-bit integer is bound as SQL_C_UTINYINT and inserted
    Then The value should be read back as double

  @odbc_e2e
  Scenario: should bind SQL_C_UBIGINT to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When A large unsigned 64-bit integer is bound as SQL_C_UBIGINT and inserted
    Then The value should round to double precision when read back

  @odbc_e2e
  Scenario: should bind SQL_C_STINYINT to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When A signed 8-bit integer at minimum value is bound as SQL_C_STINYINT and inserted
    Then The value should be read back as double

  @odbc_e2e
  Scenario: should bind SQL_C_USHORT to SQL_DOUBLE and read back
    Given Snowflake client is logged in
    When An unsigned 16-bit integer at maximum value is bound as SQL_C_USHORT and inserted
    Then The value should be read back as double

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG zero to SQL_DOUBLE
    Given Snowflake client is logged in
    When Zero is bound as SQL_C_SLONG and inserted
    Then The value should be read back as double zero

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG with NULL indicator to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_NULL_DATA is used as the length/indicator for the bound parameter
    Then The column should be NULL when read back
