@python
Feature: Arrow fetch methods (Python-specific)

  # =========================================================================== #
  #                         fetch_arrow_all                                     #
  # =========================================================================== #

  @python_e2e
  Scenario: should fetch typed rows with nulls as pyarrow Table
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR, score FLOAT, active BOOLEAN) exists
    And Rows [1, "Alice", 9.5, TRUE], [2, NULL, NULL, FALSE] are inserted
    When Query "SELECT * FROM {table} ORDER BY id" is executed
    And fetch_arrow_all is called
    Then The result should be a pyarrow.Table with 2 rows
    And Row 1 should contain [1, "Alice", 9.5, True]
    And Row 2 should contain [2, NULL, NULL, False]

  @python_e2e
  Scenario: should return None from fetch_arrow_all for empty result set
    Given Snowflake client is logged in
    When Query "SELECT 1 AS id WHERE 1=0" is executed
    And fetch_arrow_all is called
    Then The result should be None

  # =========================================================================== #
  #                        fetch_arrow_batches                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should yield multiple arrow batches for large result set
    Given Snowflake client is logged in
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And fetch_arrow_batches is called
    Then More than one batch should be yielded
    And Each element should be a pyarrow.Table
    And The total row count across all batches should be 100000
