@core_e2e @odbc_e2e @python_e2e
Feature: Large Result Set

  @core_e2e @odbc_e2e @python_e2e
  Scenario: should process one million row result set
    Given Snowflake client is logged in
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
    Then there are 1000000 numbered sequentially rows returned

  # For this query the initial chunk is empty, this test verifies that we handle this situation
  @core_e2e
  Scenario: should process ten thousand string rows when initial chunk is empty
    Given Snowflake client is logged in
    When Query "select L_COMMENT from SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM limit 10000" is executed
    Then there are 10000 rows returned
