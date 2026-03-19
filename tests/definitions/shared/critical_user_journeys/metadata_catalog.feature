@python
Feature: Metadata/Catalog Operations

  Metadata inspection via cursor.description, SHOW, and DESCRIBE.
  Used by SQLAlchemy for reflection, snowflake-cli for object management,
  Snowfort for result classification.

  # ============================================================================
  # SHOW AND DESCRIBE
  # ============================================================================

  @python_e2e
  Scenario: should return metadata via SHOW TABLES
    Given Snowflake client is logged in
    And A temporary table "meta_test" with diverse column types exists
    When SHOW TABLES LIKE 'meta_test' is executed
    Then 1 row should be returned with the table name

  @python_e2e
  Scenario: should return column metadata via DESCRIBE TABLE
    Given Snowflake client is logged in
    And A temporary table "meta_test" with diverse column types exists
    When DESCRIBE TABLE meta_test is executed
    Then 6 rows should be returned with correct column names and types

  @python_e2e
  Scenario: should return cursor description with correct column metadata after select
    Given Snowflake client is logged in
    And A temporary table "meta_test" with diverse column types exists
    When SELECT on the table is executed with WHERE 1=0
    Then cursor.description should have entries for each column with correct names

  @python_e2e
  Scenario: should return cursor description for ad hoc select
    Given Snowflake client is logged in
    When "SELECT 42 AS num, 'hello' AS str, TRUE AS flag" is executed
    Then cursor.description should have 3 entries: NUM, STR, FLAG

  @python_e2e
  Scenario: should return none for cursor description before execute
    Given Snowflake client is logged in
    When A new cursor is created without executing any query
    Then cursor.description should be None
