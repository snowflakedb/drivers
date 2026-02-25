@core
Feature: JSON Result Set

  @core_e2e
  Scenario: should return arrow even if JSON result set is returned for simple types
    Given Snowflake client is logged in
    And Query result format is forced to JSON
    When Query "SELECT 'abc', 123" is executed
    Then all values are deserialized correctly

  # TODO add a test for larger result set with chunks

  # TODO add a test for all possible data types
  # can we modify this test later to return other data types as well? or do we need a separate test for that?
