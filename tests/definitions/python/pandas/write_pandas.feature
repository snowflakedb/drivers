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

  # =========================================================================
  # Validation (errors raised before any Snowflake interaction)
  # =========================================================================

  @python_e2e
  Scenario: should raise ProgrammingError when database is set without schema
    Given Snowflake client is logged in
    When write_pandas is called with database but no schema
    Then ProgrammingError should be raised

  @python_e2e
  Scenario: should raise ProgrammingError for invalid compression
    Given Snowflake client is logged in
    When write_pandas is called with an unsupported compression value
    Then ProgrammingError should be raised

  @python_e2e
  Scenario: should raise ValueError for invalid table type
    Given Snowflake client is logged in
    When write_pandas is called with an invalid table_type
    Then ValueError should be raised

  @python_e2e
  Scenario: should emit UserWarning for tz-aware columns without use_logical_type
    Given Snowflake client is logged in
    And A DataFrame with a tz-aware datetime column
    When write_pandas is called without use_logical_type=True
    Then UserWarning about timezone should be emitted

  @python_e2e
  Scenario: should emit UserWarning for non-standard DataFrame index
    Given Snowflake client is logged in
    And A DataFrame with a string index
    When write_pandas is called with the non-standard index DataFrame
    Then UserWarning about non-standard index should be emitted

  @python_e2e
  Scenario: should handle invalid iceberg config keys
    Given Snowflake client is logged in
    When write_pandas is called with iceberg_config containing invalid keys
    Then ProgrammingError should be raised
