@python
Feature: Connection properties

  @python_e2e
  Scenario: should return valid snowflake version string
    Given Snowflake client is logged in
    When The snowflake_version property is accessed
    Then the version should be a non-empty string matching a version pattern

  @python_e2e
  Scenario: should match current version query
    Given Snowflake client is logged in
    When The snowflake_version property is accessed
    And CURRENT_VERSION() is queried directly
    Then the property value should match the direct query result

  @python_e2e
  Scenario: should return cached result
    Given Snowflake client is logged in
    When The snowflake_version property is accessed twice
    Then both accesses should return the same value
