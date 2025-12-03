@core
Feature: Async execution

  @core_e2e
  Scenario: should process async query result
    Given Snowflake client is logged in with async engine enabled
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed
    Then there are 10000 numbered sequentially rows returned
    And Statement should be released
