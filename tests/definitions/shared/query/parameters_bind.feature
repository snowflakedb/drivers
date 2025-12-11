@core
Feature: Parameter binding

  @core_e2e
  Scenario: should bind single parameter to statement
    Given Snowflake client is logged in
    And A statement is created
    When Query with single parameter is executed
    Then Query execution should return the bound parameter value
    And Statement should be released

  @core_e2e
  Scenario: should bind multiple parameters to statement
    Given Snowflake client is logged in
    And A statement is created
    When Query with multiple parameters is executed
    Then Query execution should return the bound parameter values
    And Statement should be released
