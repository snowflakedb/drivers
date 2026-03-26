@python
Feature: Query status

  @python_e2e
  Scenario: should return success status for completed query
    Given Snowflake client is logged in
    When Query "SELECT 1" is executed
    And Query status is retrieved by query ID
    Then the query status should indicate success
    And the query should not be indicated as still running
    And the query should not be indicated as an error

  @python_e2e
  Scenario: should return error status for failed query
    Given Snowflake client is logged in
    When An invalid query is executed and the query ID is captured
    And Query status is retrieved by query ID
    Then the query status should indicate an error
    And the query should not be indicated as still running

  @python_e2e
  Scenario: should indicate still running for in-progress query
    Given Snowflake client is logged in
    When A long-running query is submitted asynchronously
    And Query status is retrieved immediately
    Then the query status should indicate still running
    And the query should not be indicated as an error

  @python_e2e
  Scenario: should raise error when retrieving status with invalid query ID
    Given Snowflake client is logged in
    When Query status is retrieved for a non-existent query ID
    Then An error should be returned
