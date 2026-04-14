@python
Feature: write_pandas (Python-specific)

  @python_e2e
  Scenario: should write a DataFrame to a pre-created table and read it back
    Given Snowflake client is logged in
    And A temporary table with columns name STRING and score INT exists
    When write_pandas is called with the sample DataFrame
    Then write_pandas should return success with correct chunk and row counts
    And SELECT from the table should return all original rows

  @python_e2e
  Scenario: should auto-create a table from DataFrame schema
    Given Snowflake client is logged in
    When write_pandas is called with auto_create_table=True and table_type="temp"
    Then write_pandas should return success with correct chunk and row counts
    And SELECT from the table should return all original rows

  @python_e2e
  Scenario: should overwrite existing data with new data
    Given Snowflake client is logged in
    And A temporary table with columns name STRING and score INT exists
    And The table contains initial data
    When write_pandas is called with new data and overwrite=True
    Then write_pandas should return success with correct chunk and row counts
    And The table should contain only the new data

  @python_e2e
  Scenario: should write DataFrame in multiple chunks
    Given Snowflake client is logged in
    And A temporary table with columns name STRING and score INT exists
    When write_pandas is called with chunk_size=2
    Then write_pandas should return 3 chunks for a 5-row DataFrame
    And All original rows should be present in the table

  @python_e2e
  Scenario: should round-trip multiple data types through write_pandas
    Given Snowflake client is logged in
    When write_pandas is called with a multi-type DataFrame using auto_create_table=True and use_logical_type=True
    Then write_pandas should return success with correct chunk and row counts
    And All values should match the original data including timestamps
