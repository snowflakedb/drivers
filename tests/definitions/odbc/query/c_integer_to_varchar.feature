@odbc
Feature: ODBC SQLBindParameter C integer types to VARCHAR conversion
  # Tests for binding integer C types (SQL_C_LONG, SQL_C_SLONG, SQL_C_ULONG,
  # SQL_C_SHORT, SQL_C_SSHORT, SQL_C_USHORT, SQL_C_SBIGINT, SQL_C_UBIGINT,
  # SQL_C_TINYINT, SQL_C_STINYINT, SQL_C_UTINYINT) to SQL_VARCHAR.

  @odbc_e2e
  Scenario: should bind SQL_C_LONG to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind negative SQL_C_SLONG to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_ULONG to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SSHORT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SHORT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_USHORT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_UBIGINT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_STINYINT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_TINYINT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_UTINYINT to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG INT_MIN to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG INT_MAX to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG zero to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT LLONG_MIN to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT LLONG_MAX to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string

  @odbc_e2e
  Scenario: should bind SQL_C_UBIGINT ULLONG_MAX to SQL_VARCHAR.
    Given Snowflake client is logged in
    When the C type value is bound as a string SQL type and SELECT ? is executed
    Then the result should be the expected string
