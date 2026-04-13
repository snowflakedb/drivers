@odbc
Feature: ODBC C char types to SQL string conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_CHAR "hello world" is bound to SQL_VARCHAR and inserted
    Then the value is read back as "hello world"

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR empty string to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_CHAR empty string is bound to SQL_VARCHAR and inserted
    Then the value is read back as empty

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR to SQL_VARCHAR and read back
    Given Snowflake client is logged in
    When SQL_C_WCHAR "test" is bound to SQL_VARCHAR and inserted
    Then the value is read back as "test"

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_VARCHAR
    Given Snowflake client is logged in
    When SQL_C_CHAR is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
