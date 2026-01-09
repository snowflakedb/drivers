@odbc @python @jdbc
Feature: Empty Result Handling

  When a query returns zero rows (e.g., "SELECT 1 WHERE FALSE"), the driver
  should handle this gracefully and return an empty result set rather than
  throwing an error.

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should return empty result when query produces no rows
    Given Snowflake client is logged in
    When Query "SELECT 1 WHERE FALSE" is executed
    Then empty result set is returned

