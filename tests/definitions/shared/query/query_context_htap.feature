@python
Feature: Query context cache E2E with hybrid tables

  @python_e2e
  Scenario: should preserve schema after SELECT under HTAP optimization
    Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
    When the client creates a new schema and executes SELECT
    Then the connection still reports the new schema

  @python_e2e
  Scenario: should preserve database after SELECT under HTAP optimization
    Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
    When the client creates a new database and executes SELECT
    Then the connection still reports the new database

  @python_e2e
  Scenario: should preserve role after SELECT under HTAP optimization
    Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
    When the client switches to a different role and executes SELECT
    Then the connection still reports the switched role

  @python_e2e
  Scenario: should preserve session parameter after SELECT under HTAP optimization
    Given the account has ENABLE_SNOW_654741_FOR_TESTING enabled
    When the client changes DATE_OUTPUT_FORMAT and executes SELECT
    Then the session parameter still reflects the changed value

  @python_e2e
  Scenario: should operate on hybrid tables across multiple databases
    Given a connection to Snowflake
    When the client creates hybrid tables in two databases and inserts rows
    Then selecting from each database returns the correct rows after switching back
