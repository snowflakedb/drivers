@python
Feature: Full Table Lifecycle (DDL + DML)

  Core CRUD workflow with multi-type tables, DESCRIBE verification,
  UPDATE/DELETE with rowcount. Used by every consumer.

  # ============================================================================
  # TABLE CREATION AND METADATA
  # ============================================================================

  @python_e2e
  Scenario: should create table with multiple column types and verify via DESCRIBE
    Given Snowflake client is logged in
    When A temporary table "lifecycle_test" is created with columns (id INT NOT NULL, name VARCHAR(100), active BOOLEAN, score FLOAT, amount NUMBER(10,2))
    Then DESCRIBE TABLE should return 5 columns with correct names and types

  # ============================================================================
  # INSERT AND SELECT
  # ============================================================================

  @python_e2e
  Scenario: should insert multiple rows and verify via SELECT
    Given Snowflake client is logged in
    And A temporary table "lifecycle_test" with typed columns exists
    When 3 rows are inserted with diverse types
    Then SELECT with ORDER BY should return 3 rows with correct values and types
    And Insert rowcount should be 3

  @python_e2e
  Scenario: should filter rows with WHERE clause
    Given Snowflake client is logged in
    And A temporary table "lifecycle_test" with test data exists
    When SELECT with WHERE active = TRUE is executed
    Then Only rows where active is TRUE should be returned

  # ============================================================================
  # UPDATE AND DELETE
  # ============================================================================

  @python_e2e
  Scenario: should update row and verify rowcount
    Given Snowflake client is logged in
    And A temporary table "lifecycle_test" with test data exists
    When UPDATE SET name = 'Alice Updated' WHERE id = 1 is executed
    Then Update rowcount should be 1
    And SELECT should show the updated name for id=1

  @python_e2e
  Scenario: should delete row and verify rowcount
    Given Snowflake client is logged in
    And A temporary table "lifecycle_test" with test data exists
    When DELETE WHERE active = FALSE is executed
    Then Delete rowcount should be 1
    And SELECT COUNT(*) should reflect the deletion
