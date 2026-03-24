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
    When A temporary table is created with columns (id INT NOT NULL, name VARCHAR(100), active BOOLEAN, score FLOAT, amount NUMBER(10,2))
    Then DESCRIBE TABLE should return 5 columns with correct names and types

  # ============================================================================
  # FILTERING
  # ============================================================================

  @python_e2e
  Scenario: should filter rows with WHERE clause
    Given Snowflake client is logged in
    And A temporary table with test data exists
    When SELECT with WHERE active = TRUE is executed
    Then Only rows where active is TRUE should be returned
