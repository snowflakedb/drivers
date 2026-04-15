@odbc
Feature: ODBC C time type to SQL time conversion via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME to SQL_TYPE_TIME and read back
    Given Snowflake client is logged in
    When SQL_C_TYPE_TIME 14:30:45 is bound to SQL_TYPE_TIME and inserted
    Then the value is read back as 14:30:45

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME with NULL indicator to SQL_TYPE_TIME
    Given Snowflake client is logged in
    When SQL_C_TYPE_TIME is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
