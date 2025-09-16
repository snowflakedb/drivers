@core_e2e @odbc_e2e @python_e2e
Feature: Large Result Set

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should process one million row result set
    Given Snowflake client is logged in
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
    Then there are 1000000 numbered sequentially rows returned
