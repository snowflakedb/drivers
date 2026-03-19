@python
Feature: Pandas fetch methods (Python-specific)

  # =========================================================================== #
  #                         fetch_pandas_all                                    #
  # =========================================================================== #

  @python_e2e
  Scenario: should fetch typed rows with nulls as pandas DataFrame
    Given Snowflake client is logged in
    And A temporary table with columns (id NUMBER, name VARCHAR, score FLOAT, active BOOLEAN) exists
    And Rows [1, "Alice", 9.5, TRUE], [2, NULL, NULL, FALSE] are inserted
    When Query "SELECT * FROM {table} ORDER BY id" is executed
    And fetch_pandas_all is called
    Then The result should be a pandas.DataFrame with 2 rows
    And Row 1 should contain [1, "Alice", 9.5, True]
    And Row 2 should contain [2, None/NaN, None/NaN, False]

  @python_e2e
  Scenario: should return empty pandas DataFrame for empty result set
    Given Snowflake client is logged in
    When Query "SELECT 1 AS id WHERE 1=0" is executed
    And fetch_pandas_all is called
    Then The result should be a pandas.DataFrame with 0 rows

  # =========================================================================== #
  #                       fetch_pandas_batches                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should yield multiple pandas DataFrames for large result set
    Given Snowflake client is logged in
    When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
    And fetch_pandas_batches is called
    Then More than one DataFrame should be yielded
    And Each element should be a pandas.DataFrame
    And The total row count across all DataFrames should be 100000
