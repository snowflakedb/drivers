@core
Feature: Async execution

  @core_e2e
  Scenario: should process async query result
    Given Snowflake client is logged in with async engine enabled
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 10000)) v ORDER BY id" is executed
    Then there are 10000 numbered sequentially rows returned
    And Statement should be released

  @core_e2e
  Scenario: should match blocking results when async execution enabled
    Given Snowflake client is logged in
    And Statement A has async execution disabled
    And Statement B has async execution enabled
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000)) v ORDER BY id" is executed on both statements
    Then both result sets have identical sequential ids
    And Both statements should be released

  @core_e2e
  Scenario: should use async by default when no execution mode specified
    Given Snowflake client is logged in
    And Statement has no async execution setting
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100)) v ORDER BY id" is executed
    Then there are 100 numbered sequentially rows returned
    And Statement should be released

