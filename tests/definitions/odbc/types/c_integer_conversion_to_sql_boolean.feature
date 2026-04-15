@odbc
Feature: ODBC C integer types to SQL boolean conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG one to SQL_BIT via integer
    Given Snowflake client is logged in
    When SQL_C_SLONG 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG zero to SQL_BIT via integer
    Given Snowflake client is logged in
    When SQL_C_SLONG 0 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 0

  @odbc_e2e
  Scenario: should bind SQL_C_SBIGINT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_SBIGINT 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_SSHORT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_SSHORT 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_UTINYINT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_UTINYINT 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_ULONG to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_ULONG 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_STINYINT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_STINYINT 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_USHORT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_USHORT 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_UBIGINT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_UBIGINT 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG with NULL indicator to SQL_BIT via integer
    Given Snowflake client is logged in
    When SQL_C_SLONG is bound with SQL_NULL_DATA and inserted
    Then the column is NULL when fetched
