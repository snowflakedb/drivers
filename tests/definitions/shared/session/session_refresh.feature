@core
Feature: Session Token Refresh
  As a driver user
  I want the driver to automatically refresh expired session tokens
  So that long-running applications don't fail due to token expiration

  @core_e2e
  Scenario: should maintain session across multiple queries
    # E2E test - verifies session management works end-to-end
    Given Snowflake client is logged in
    When we execute multiple queries
    Then each query should succeed with the correct result

  @core_e2e
  Scenario: should execute queries with delay between them
    # E2E test - verifies session remains valid over time
    Given Snowflake client is logged in
    When we execute queries with delays between them
    Then each query should succeed
