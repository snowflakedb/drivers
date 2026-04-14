@odbc
Feature: ODBC C char types to SQL real conversions via parameter binding

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR float string to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_CHAR "3.14" is bound to SQL_DOUBLE and inserted
    Then the value is read back as approximately 3.14

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR integer string to SQL_REAL
    Given Snowflake client is logged in
    When SQL_C_CHAR "100" is bound to SQL_REAL and inserted
    Then the value is read back as 100.0

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to SQL_FLOAT synonym
    Given Snowflake client is logged in
    When SQL_C_CHAR "1.23" is bound to SQL_FLOAT and inserted
    Then the value is read back as approximately 1.23

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to FLOAT4 column
    Given Snowflake client is logged in
    When SQL_C_CHAR "5.5" is bound to SQL_DOUBLE and inserted into a FLOAT4 column
    Then the value is read back as approximately 5.5

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to FLOAT8 column
    Given Snowflake client is logged in
    When SQL_C_CHAR "9.81" is bound to SQL_DOUBLE and inserted into a FLOAT8 column
    Then the value is read back as approximately 9.81

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to DOUBLE PRECISION column
    Given Snowflake client is logged in
    When SQL_C_CHAR "2.22" is bound to SQL_DOUBLE and inserted into a DOUBLE PRECISION column
    Then the value is read back as approximately 2.22

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR to REAL column
    Given Snowflake client is logged in
    When SQL_C_CHAR "7.77" is bound to SQL_REAL and inserted into a REAL column
    Then the value is read back as approximately 7.77

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR float string to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_WCHAR "2.71" is bound to SQL_DOUBLE and inserted
    Then the value is read back as approximately 2.71

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR integer string to SQL_REAL
    Given Snowflake client is logged in
    When SQL_C_WCHAR "200" is bound to SQL_REAL and inserted
    Then the value is read back as 200.0

  @odbc_e2e
  Scenario: should bind SQL_C_CHAR with NULL indicator to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_CHAR is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL

  @odbc_e2e
  Scenario: should bind SQL_C_WCHAR with NULL indicator to SQL_DOUBLE
    Given Snowflake client is logged in
    When SQL_C_WCHAR is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
