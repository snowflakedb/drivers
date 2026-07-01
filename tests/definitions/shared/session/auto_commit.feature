@jdbc
Feature: Connection autocommit

  Each wrapper exposes per-connection autocommit. The shared core RPC
  `connection_set_autocommit` toggles the server-side setting; on read,
  drivers report the current setting through their native API.

  @jdbc_e2e
  Scenario: should report autocommit as disabled after it was disabled on the connection
    Given Snowflake client is logged in
    When autocommit is disabled on the connection
    Then the autocommit setting reports as disabled

  @jdbc_e2e
  Scenario: should report autocommit as enabled after it was re-enabled on the connection
    Given Snowflake client is logged in
    And autocommit was disabled on the connection
    When autocommit is enabled on the connection
    Then the autocommit setting reports as enabled

  @jdbc_e2e
  Scenario: should discard uncommitted inserts on rollback
    Given Snowflake client is logged in
    And a transient table exists in the test schema
    When the writer disables autocommit, inserts a row, and rolls back
    Then a reader session sees zero rows

  @jdbc_e2e
  Scenario: should publish committed inserts to other sessions
    Given Snowflake client is logged in
    And a transient table exists in the test schema
    When the writer disables autocommit, inserts a row, and commits
    Then a reader session sees one row
