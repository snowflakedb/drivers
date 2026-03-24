@core
Feature: Distributed fetch

  @core_e2e
  Scenario: distributed fetch simple query
    Given Snowflake client is logged in
    When Query "SELECT 42 AS answer, 'hello' AS greeting" is executed
    Then result chunks should contain at least one inline chunk
    And fetching the inline chunk should return 1 row with 2 columns
    And Statement should be released

  @core_e2e
  Scenario: distributed fetch large result produces multiple chunks
    Given Snowflake client is logged in
    When Large query generating 500000 rows is executed
    Then result chunks should contain at least 2 chunks
    And result chunks should contain at least one remote chunk
    And fetching all chunks should return 500000 total rows
    And Statement should be released
