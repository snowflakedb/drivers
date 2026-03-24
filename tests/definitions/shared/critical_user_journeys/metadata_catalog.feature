@python
Feature: Metadata/Catalog Operations

  Metadata inspection via SHOW and DESCRIBE.
  Used by SQLAlchemy for reflection, snowflake-cli for object management,
  Snowfort for result classification.

  # ============================================================================
  # SHOW AND DESCRIBE
  # ============================================================================

  @python_e2e
  Scenario: should return metadata via SHOW TABLES
    Given Snowflake client is logged in
    And A temporary table with diverse column types exists
    When SHOW TABLES LIKE 'meta_show' is executed
    Then 1 row should be returned with the table name

  @python_e2e
  Scenario: should return column metadata via DESCRIBE TABLE
    Given Snowflake client is logged in
    And A temporary table with diverse column types exists
    When DESCRIBE TABLE meta_describe is executed
    Then 6 rows should be returned with correct column names and types
