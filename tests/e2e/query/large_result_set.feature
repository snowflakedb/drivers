@core @odbc @python
Feature: Large Result Set

  @core @odbc @python
  Scenario: should process one million row result set
    Given Snowflake client is logged in
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
    Then there are 1000000 numbered sequentially rows returned
