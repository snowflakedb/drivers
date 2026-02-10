@python
Feature: Binary LOB (Large Object) handling
  # Snowflake LOB feature supports these large BINARY values:
  #   - Default (pre-2025_03 bundle): Maximum 8 MB (8,388,608 bytes)
  #   - With 2025_03 bundle enabled: Maximum 64 MB (67,108,864 bytes)
  # Reference: https://docs.snowflake.com/en/release-notes/bcr-bundles/2025_03/bcr-1942

  @python_e2e
  Scenario: should handle maximum default binary size
    # Default maximum binary size is 8MB (8,388,608 bytes)
    # This is the limit before enabling the 2025_03 behavior change bundle
    Given Snowflake client is logged in
    And Table with BINARY column exists
    When Binary value of 8MB size (8,388,608 bytes) is inserted
    And Query "SELECT * FROM {table}" is executed
    Then the retrieved value size should be 8MB (8,388,608 bytes)
    And data integrity should be maintained

  @python_e2e
  Scenario: should handle extended maximum binary size
    # Extended maximum binary size is 64MB (67,108,864 bytes)
    Given Snowflake client is logged in
    And Table with BINARY(67108864) column exists
    When Binary value of 64MB size (67,108,864 bytes) is inserted
    And Query "SELECT * FROM {table}" is executed
    Then the retrieved value size should be 64MB (67,108,864 bytes)
    And data integrity should be maintained