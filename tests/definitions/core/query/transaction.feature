@core
Feature: Transaction

  @core_e2e
  Scenario: should commit transaction
    Given a connected Snowflake client with autocommit disabled
    When a row is inserted and committed
    Then the row is visible after commit

  @core_e2e
  Scenario: should rollback transaction
    Given a connected Snowflake client with autocommit disabled and a committed row
    When a second row is inserted and rolled back
    Then only the first committed row remains

  @core_e2e
  Scenario: should commit with no pending changes
    Given a connected Snowflake client with autocommit disabled and an empty table
    When commit is called with no pending changes
    Then the table remains empty and no error occurs

  @core_e2e
  Scenario: should rollback with no pending changes
    Given a connected Snowflake client with autocommit disabled and an empty table
    When rollback is called with no pending changes
    Then the table remains empty and no error occurs

  @core_e2e
  Scenario: should commit multiple inserts in single transaction
    Given a connected Snowflake client with autocommit disabled
    When multiple rows are inserted and committed in a single transaction
    Then all rows are visible after commit

  @core_e2e
  Scenario: should commit and rollback with autocommit enabled
    Given a connected Snowflake client with autocommit enabled
    When commit and rollback are called with autocommit enabled
    Then the auto-committed row is still visible
