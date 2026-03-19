@odbc
Feature: Parameter array statement attributes
  # Tests for SQL_ATTR_PARAMSET_SIZE, SQL_ATTR_PARAM_BIND_TYPE, SQL_ATTR_PARAM_BIND_OFFSET_PTR,
  # SQL_ATTR_PARAM_STATUS_PTR, SQL_ATTR_PARAMS_PROCESSED_PTR, and SQL_ATTR_PARAM_OPERATION_PTR

  @odbc_e2e
  Scenario: SQL_ATTR_PARAMSET_SIZE default value is 1.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAMSET_SIZE is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value 1

  @odbc_e2e
  Scenario: SQL_ATTR_PARAMSET_SIZE can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAMSET_SIZE is set to 5
    Then it should return SQL_SUCCESS and the retrieved value should be 5

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_BIND_TYPE default value is SQL_PARAM_BIND_BY_COLUMN.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_BIND_TYPE is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value 0

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_BIND_TYPE can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_BIND_TYPE is set to a row size
    Then it should return SQL_SUCCESS and the retrieved value should match

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_BIND_OFFSET_PTR default value is NULL.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_BIND_OFFSET_PTR is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value NULL

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_BIND_OFFSET_PTR can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_BIND_OFFSET_PTR is set to a pointer
    Then it should return SQL_SUCCESS and the retrieved pointer should match

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_STATUS_PTR default value is NULL.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_STATUS_PTR is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value NULL

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_STATUS_PTR can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_STATUS_PTR is set to a pointer
    Then it should return SQL_SUCCESS and the retrieved pointer should match

  @odbc_e2e
  Scenario: SQL_ATTR_PARAMS_PROCESSED_PTR default value is NULL.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAMS_PROCESSED_PTR is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value NULL

  @odbc_e2e
  Scenario: SQL_ATTR_PARAMS_PROCESSED_PTR can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAMS_PROCESSED_PTR is set to a pointer
    Then it should return SQL_SUCCESS and the retrieved pointer should match

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_OPERATION_PTR default value is NULL.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_OPERATION_PTR is queried on a fresh statement
    Then it should return SQL_SUCCESS and the value NULL

  @odbc_e2e
  Scenario: SQL_ATTR_PARAM_OPERATION_PTR can be set and retrieved.
    Given Snowflake client is logged in
    When SQL_ATTR_PARAM_OPERATION_PTR is set to a pointer
    Then it should return SQL_SUCCESS and the retrieved pointer should match

  @odbc_e2e
  Scenario: PARAMSET_SIZE greater than 1 executes multiple parameter sets.
    Given Snowflake client is logged in
    When SQLExecDirect is called with PARAMSET_SIZE set to 3 and an array of 3 integer values
    Then it should return SQL_SUCCESS and insert all 3 rows

  @odbc_e2e
  Scenario: PARAMS_PROCESSED_PTR is written with the number of parameter sets after execution.
    Given Snowflake client is logged in
    When SQLExecDirect is called with PARAMSET_SIZE set to 3 and PARAMS_PROCESSED_PTR bound
    Then PARAMS_PROCESSED_PTR should contain 3 after execution

  @odbc_e2e
  Scenario: PARAM_STATUS_PTR is written with SQL_PARAM_SUCCESS for each row after execution.
    Given Snowflake client is logged in
    When SQLExecDirect is called with PARAMSET_SIZE set to 3 and PARAM_STATUS_PTR bound
    Then each slot of PARAM_STATUS_PTR should contain SQL_PARAM_SUCCESS after execution
