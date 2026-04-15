@odbc
Feature: ODBC C integer types to SQL string conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_SLONG 42 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "42"

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_SBIGINT 9999999999 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "9999999999"

  @odbc_e2e
  Scenario: should bind SQL_C_SSHORT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_SSHORT -32768 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "-32768"

  @odbc_e2e
  Scenario: should bind SQL_C_UTINYINT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_UTINYINT 255 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "255"

  @odbc_e2e
  Scenario: should bind SQL_C_ULONG to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_ULONG 4000000000 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "4000000000"

  @odbc_e2e
  Scenario: should bind SQL_C_STINYINT negative to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_STINYINT -128 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "-128"

  @odbc_e2e
  Scenario: should bind SQL_C_USHORT to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_USHORT 65535 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "65535"

  @odbc_e2e
  Scenario: should bind SQL_C_UBIGINT max to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_UBIGINT max is bound to SQL_VARCHAR and inserted
    Then the value is read back correctly

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG zero to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_SLONG 0 is bound to SQL_VARCHAR and inserted
    Then the value is read back as "0"

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG with NULL indicator to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_SLONG is bound with SQL_NULL_DATA to SQL_VARCHAR and inserted
    Then the stored value should be NULL
