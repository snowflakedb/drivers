@python
Feature: Transaction Control

  Transaction management including autocommit toggling, commit, and rollback.
  Used by SQLAlchemy (default autocommit=False), snowflake-cli, Snowpark, and Snowfort.

  # ============================================================================
  # ROLLBACK
  # ============================================================================

  @python_e2e
  Scenario: should rollback uncommitted insert when autocommit is off
    Given Snowflake client is logged in
    And A test table "tx_test" with columns (id INT, name VARCHAR) exists
    And Autocommit is set to OFF
    When A row (1, 'uncommitted') is inserted into "tx_test"
    And The row is visible within the session via SELECT
    And ROLLBACK is executed
    Then The table "tx_test" should have 0 rows

  # ============================================================================
  # COMMIT
  # ============================================================================

  @python_e2e
  Scenario: should persist committed insert when autocommit is off
    Given Snowflake client is logged in
    And A test table "tx_test" with columns (id INT, name VARCHAR) exists
    And Autocommit is set to OFF
    When A row (2, 'committed') is inserted into "tx_test"
    And COMMIT is executed
    Then The table "tx_test" should have 1 row with id=2

  # ============================================================================
  # AUTOCOMMIT ON
  # ============================================================================

  @python_e2e
  Scenario: should auto-persist insert when autocommit is on
    Given Snowflake client is logged in
    And A test table "tx_test" with columns (id INT, name VARCHAR) exists
    And Autocommit is set to ON
    When A row (3, 'auto') is inserted into "tx_test"
    Then The row should be immediately visible without explicit commit
