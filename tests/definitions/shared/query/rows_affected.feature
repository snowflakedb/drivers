@dotnet
Feature: Rows affected

  @dotnet_e2e
  Scenario: should report the row count of a SELECT as rows affected
    Given Snowflake client is logged in
    When Query "SELECT seq8() FROM TABLE(GENERATOR(ROWCOUNT => 5)) v" is executed as a non-query and as a reader
    Then the non-query result is 5
    And the reader reports 5 records affected

  @dotnet_e2e
  Scenario: should report the row count of a DESCRIBE as rows affected
    Given Snowflake client is logged in
    And a temporary table with 3 columns exists
    When Query "DESCRIBE TABLE <table>" is executed as a non-query and as a reader
    Then the non-query result is 3
    And the reader reports 3 records affected

  @dotnet_e2e
  Scenario: should report minus one records affected for DDL statement
    Given Snowflake client is logged in
    When a CREATE TEMPORARY TABLE statement is executed as a non-query and another as a reader
    Then the non-query result is 0
    And the reader reports -1 records affected

  @dotnet_e2e
  Scenario: should report the number of unload status rows for COPY INTO a stage
    Given Snowflake client is logged in
    And a temporary stage and a temporary table with 1000 rows exist
    When the table is unloaded into one file of the stage as a non-query
    Then the non-query result is 1

  @dotnet_e2e
  Scenario: should report the number of load status rows for COPY INTO a table
    Given Snowflake client is logged in
    And a temporary stage holding one file with 1000 rows and an empty temporary table exist
    When the file is copied into the table as a non-query
    Then the non-query result is 1
    And the table holds 1000 rows
