@python
Feature: Object Discovery (SHOW/DESCRIBE/DROP)

  Object management via SHOW, DESCRIBE, and DROP commands.
  Used by snowflake-cli for all object management, SQLAlchemy for metadata
  reflection, Snowfort extensively.

  @python_e2e
  Scenario: should discover table via SHOW and DESCRIBE
    Given Snowflake client is logged in
    And A table "e2e_discovery_test" with columns (id INT NOT NULL, name VARCHAR(100), val NUMBER(10,2)) exists
    When SHOW TABLES LIKE 'e2e_discovery_test' is executed
    Then 1 row should be returned
    When DESCRIBE TABLE e2e_discovery_test is executed
    Then 3 columns with correct names and types should be returned

  @python_e2e
  Scenario: should discover view via SHOW
    Given Snowflake client is logged in
    And A table "e2e_discovery_test" exists
    And A view "e2e_discovery_view" is created on the table
    When SHOW VIEWS LIKE 'e2e_discovery_view' is executed
    Then 1 row should be returned

  @python_e2e
  Scenario: should verify objects are gone after DROP
    Given Snowflake client is logged in
    And A table "e2e_discovery_test" and view "e2e_discovery_view" exist
    When Both objects are dropped
    Then SHOW TABLES LIKE 'e2e_discovery_test' should return 0 rows
    And SHOW VIEWS LIKE 'e2e_discovery_view' should return 0 rows
