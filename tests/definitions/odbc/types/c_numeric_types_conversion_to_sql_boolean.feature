@odbc
Feature: ODBC numeric C types to SQL_BIT (BOOLEAN) conversions

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG one to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_SLONG 1 is bound to SQL_BIT and inserted into BOOLEAN
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG zero to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_SLONG 0 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 0

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE one to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_DOUBLE 1.0 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_DOUBLE zero to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_DOUBLE 0.0 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 0

  @odbc_e2e
  Scenario: should bind SQL_C_BIT to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_BIT 1 is bound to SQL_BIT and inserted
    Then the value is read back as SQL_C_BIT 1

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG negative to SQL_BIT as true
    Given Snowflake client is logged in
    When SQL_C_SLONG -1 is bound to SQL_BIT and inserted
    Then a nonzero value is stored as true (SQL_C_BIT 1)

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC nonzero to SQL_BIT as true
    Given Snowflake client is logged in
    When SQL_C_NUMERIC with value 42 is bound to SQL_BIT and inserted
    Then a nonzero numeric is stored as true (SQL_C_BIT 1)

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC zero to SQL_BIT as false
    Given Snowflake client is logged in
    When SQL_C_NUMERIC with value 0 is bound to SQL_BIT and inserted
    Then a zero numeric is stored as false (SQL_C_BIT 0)

  @odbc_e2e
  Scenario: should bind SQL_C_NUMERIC negative to SQL_BIT as true
    Given Snowflake client is logged in
    When SQL_C_NUMERIC with negative value (sign=0, magnitude=7) is bound to SQL_BIT and inserted
    Then a negative numeric is stored as true (SQL_C_BIT 1)

  @odbc_e2e
  Scenario: should bind SQL_C_SLONG with NULL indicator to SQL_BIT
    Given Snowflake client is logged in
    When SQL_C_SLONG is bound with SQL_NULL_DATA and inserted
    Then the column is NULL when fetched as SQL_C_BIT
