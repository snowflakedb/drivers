@python
Feature: TIMESTAMP_LTZ type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast timestamp_ltz values to appropriate type
    # Python: Values should be cast to 'datetime' type with tzinfo set
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_LTZ" is executed
    Then All values should be returned as appropriate type
    And Values should have timezone info

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should select timestamp_ltz literals
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_LTZ, '2024-06-20 14:45:30'::TIMESTAMP_LTZ" is executed
    Then Result should contain expected timestamp values

  @python_e2e
  Scenario: should handle NULL values from literals
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
    Then Result should contain [timestamp, NULL]

  @python_e2e
  Scenario: should handle epoch timestamp
    Given Snowflake client is logged in
    When Query "SELECT '1970-01-01 00:00:00'::TIMESTAMP_LTZ" is executed
    Then Result should contain epoch timestamp

  @python_e2e
  Scenario: should handle timestamp with microseconds
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00.123456'::TIMESTAMP_LTZ" is executed
    Then Result should preserve microsecond precision

  @python_e2e
  Scenario: should download large result set with multiple chunks from GENERATOR
    Given Snowflake client is logged in
    When Query "SELECT DATEADD(second, seq8(), '2024-01-01'::TIMESTAMP_LTZ) FROM <generator>" is executed
    Then Result should contain expected number of timestamp values

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select timestamp_ltz values from table
    Given Snowflake client is logged in
    And Table with TIMESTAMP_LTZ column exists
    And Timestamp rows are inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain expected timestamp values

  @python_e2e
  Scenario: should handle NULL values from table
    Given Snowflake client is logged in
    And Table with TIMESTAMP_LTZ column exists
    And Rows with NULL and non-NULL timestamps are inserted
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [timestamp, NULL] in any order

  @python_e2e
  Scenario: should download large result set with multiple chunks from table
    Given Snowflake client is logged in
    And Table with TIMESTAMP_LTZ column exists with many rows
    When Query "SELECT col FROM <table>" is executed
    Then Result should contain expected number of timestamp values

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select timestamp_ltz using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
    Then Result should contain the bound timestamps
    When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e
  Scenario: should insert timestamp_ltz using parameter binding
    Given Snowflake client is logged in
    And Table with TIMESTAMP_LTZ column exists
    When Timestamp values are bulk-inserted using multirow binding
    Then SELECT should return the same values in any order
