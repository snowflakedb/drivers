@python
Feature: Cursor/Statement Reuse

  Cursor reuse across multiple queries with state replacement and error recovery.
  Used by Snowfort for thousands of sequential queries, Snowpark for DataFrame
  operations, CLI for interactive REPL sessions.

  @python_e2e
  Scenario: should replace cursor state on subsequent queries
    Given Snowflake client is logged in
    And A cursor is created
    When "SELECT 1 AS a" is executed
    Then The cursor should have 1 column named "A"
    When "SELECT 2 AS b, 3 AS c" is executed on the same cursor
    Then The cursor should have 2 columns named "B" and "C"

  @python_e2e
  Scenario: should reuse cursor across DDL DML and SELECT
    Given Snowflake client is logged in
    And A cursor is created
    When CREATE TEMPORARY TABLE is executed on the cursor
    And INSERT is executed on the same cursor
    And SELECT is executed on the same cursor
    Then Each operation should succeed with correct results

  @python_e2e
  Scenario: should recover cursor after error and execute successfully
    Given Snowflake client is logged in
    And A cursor is created
    When Invalid SQL is executed and the error is caught
    And "SELECT 42" is executed on the same cursor
    Then The cursor should return (42,) successfully
