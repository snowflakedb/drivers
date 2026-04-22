@odbc
Feature: ODBC SQL_C_BIT to numeric SQL type conversions

  @odbc_e2e
  Scenario: should bind SQL_C_BIT true to SQL_INTEGER
    Given Snowflake client is logged in
    When SQL_C_BIT 1 is bound to SQL_INTEGER and inserted
    Then the value is read back as SQL_C_SBIGINT 1

  @odbc_e2e
  Scenario: should bind SQL_C_BIT false to SQL_INTEGER
    Given Snowflake client is logged in
    When SQL_C_BIT 0 is bound to SQL_INTEGER and inserted
    Then the value is read back as SQL_C_SBIGINT 0

  @odbc_e2e
  Scenario: should bind SQL_C_BIT to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_BIT 1 is bound to SQL_DOUBLE and inserted into FLOAT
    Then the value is read back as SQL_C_DOUBLE 1.0

  @odbc_e2e
  Scenario: should bind SQL_C_BIT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_BIT 1 is bound to SQL_BIT and inserted into BOOLEAN
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_BIT with NULL indicator
    Given Snowflake client is logged in
    When SQL_C_BIT is bound with SQL_NULL_DATA and inserted
    Then the column is NULL
