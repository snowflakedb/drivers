@python
Feature: Stored Procedure Lifecycle

  Create, call, and drop stored procedures.
  Used by snowflake-cli for Snowpark procedure deployment, Snowfort for testing,
  Snowpark for temporary procedure creation.

  @python_e2e
  Scenario: should create call and drop a SQL stored procedure
    Given Snowflake client is logged in
    When SHOW PROCEDURES LIKE 'e2e_test_proc' is executed
    Then The result should be empty
    When A SQL stored procedure "e2e_test_proc" is created that returns 'Hello, ' || name
    Then SHOW PROCEDURES LIKE 'e2e_test_proc' should return 1 row
    When CALL e2e_test_proc('World') is executed
    Then The result should be "Hello, World"
    When The procedure is dropped
    Then SHOW PROCEDURES LIKE 'e2e_test_proc' should return 0 rows
