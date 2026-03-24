@python
Feature: Object Discovery (SHOW/DESCRIBE/DROP)

  Object management via SHOW, DESCRIBE, and DROP commands.
  Used by snowflake-cli for all object management, SQLAlchemy for metadata
  reflection, Snowfort extensively.

  @python_e2e
  Scenario: should discover table via SHOW and DESCRIBE
    Given Snowflake client is logged in
    And A table with columns (id INT NOT NULL, name VARCHAR(100), val NUMBER(10,2)) exists
    When SHOW TABLES LIKE 'disc_show_describe' is executed
    Then 1 row should be returned
    When DESCRIBE TABLE e2e_discovery_test is executed
    Then 3 columns with correct names and types should be returned

  @python_e2e
  Scenario: should discover view via SHOW
    Given Snowflake client is logged in
    And A table exists
    And A view is created on the table
    When SHOW VIEWS LIKE 'disc_view_show' is executed
    Then 1 row should be returned

  @python_e2e
  Scenario: should verify objects are gone after DROP
    Given Snowflake client is logged in
    And A table and view exist
    When Both objects are dropped
    Then SHOW TABLES should return 0 rows
    And SHOW VIEWS should return 0 rows
