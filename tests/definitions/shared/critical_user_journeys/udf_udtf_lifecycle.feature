@python
Feature: UDF/UDTF Lifecycle

  Create, call, and drop user-defined functions and table functions.
  Used by Snowpark for Python-on-Snowflake execution, snowflake-cli for deployment.

  @python_e2e
  Scenario: should create call and drop a SQL UDF
    Given Snowflake client is logged in
    When A SQL UDF "e2e_test_udf" is created that returns x * 2
    Then SELECT e2e_test_udf(21) should return 42
    When The UDF is dropped
    Then SHOW FUNCTIONS LIKE 'e2e_test_udf' should return 0 rows

  @python_e2e
  Scenario: should create call and drop a SQL UDTF
    Given Snowflake client is logged in
    When A SQL UDTF "e2e_test_udtf" is created that generates n rows
    Then SELECT * FROM TABLE(e2e_test_udtf(5)) should return 5 rows
    When The UDTF is dropped
    Then SHOW FUNCTIONS LIKE 'e2e_test_udtf' should return 0 rows
