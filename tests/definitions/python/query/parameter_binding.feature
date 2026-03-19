@python
Feature: Parameter binding (Python-specific)

  # Python-specific tests that don't apply to other wrappers.
  # Cross-wrapper parameter binding scenarios live in shared/query/parameters_bind.feature.

  @python_e2e
  Scenario: should handle both tuple and list parameter formats
    Given Snowflake client is logged in
    When Query "SELECT ?, ?" is executed with tuple parameters (1, "test")
    And Query "SELECT ?, ?" is executed with list parameters [1, "test"]
    Then Both results should be identical
