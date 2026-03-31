@odbc @python @jdbc
Feature: String LOB (Large Object) handling
  # Snowflake LOB feature supports large VARCHAR values:
  #   - Historical limit: 16 MB (16,777,216 bytes) per value
  #   - Increased LOB Size feature: up to 128 MB (134,217,728 bytes) per value

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should handle LOB string at historical 16 MB limit
    # Corner case: string at the historical LOB limit (16 MB = 16,777,216 bytes)
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    When A string of 16777216 ASCII characters is generated and inserted
    And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
    Then the result should show length 16777216
    And the returned string should exactly match the generated string

  @odbc_e2e @python_e2e @jdbc_e2e
  Scenario: should handle LOB string at maximum 128 MB limit with increased LOB size
    # Corner case: string at maximum LOB limit (128 MB) - requires Increased LOB Size feature
    Given Snowflake client is logged in
    And A temporary table with VARCHAR column is created
    When A string of 134217728 ASCII characters is generated and inserted
    And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
    Then the result should show length 134217728
    And the returned string should exactly match the generated string
