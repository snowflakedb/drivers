@python
Feature: TIMESTAMP_LTZ type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast timestamp_ltz values to appropriate type
    # Python: Values should be cast to 'datetime' type with tzinfo set
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ" is executed
    Then All values should be returned as appropriate type
    And Values should have timezone info

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select timestamp_ltz <values>
    Given Snowflake client is logged in
    When Query "SELECT <query_values>" is executed
    Then Result should contain timestamps <expected_values>

    Examples:
      | values       | query_values                                                                                      | expected_values                                   |
      | basic        | '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, '2024-06-20 14:45:30 +00:00'::TIMESTAMP_LTZ         | 2024-01-15 10:30:00 UTC, 2024-06-20 14:45:30 UTC |
      | epoch        | '1970-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ                                                       | 1970-01-01 00:00:00 UTC                           |
      | microseconds | '2024-01-15 10:30:00.123456 +00:00'::TIMESTAMP_LTZ                                                | 2024-01-15 10:30:00.123456 UTC                    |

  @python_e2e
  Scenario: should handle NULL values for timestamp_ltz
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00 +00:00'::TIMESTAMP_LTZ, NULL::TIMESTAMP_LTZ" is executed
    Then Result should contain [2024-01-15 10:30:00 UTC, NULL]

  @python_e2e
  Scenario: should download large result set with multiple chunks for timestamp_ltz
    Given Snowflake client is logged in
    When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_LTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
    Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select <values> from table for timestamp_ltz
    Given Snowflake client is logged in
    And Table with TIMESTAMP_LTZ column exists with values <insert_values>
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain timestamps <expected_values>

    Examples:
      | values | insert_values                                                         | expected_values                                   |
      | basic  | '2024-01-15 10:30:00 +00:00', '2024-06-20 14:45:30 +00:00'           | 2024-01-15 10:30:00 UTC, 2024-06-20 14:45:30 UTC |
      | epoch  | '1970-01-01 00:00:00 +00:00', '2024-01-15 10:30:00 +00:00'           | 1970-01-01 00:00:00 UTC, 2024-01-15 10:30:00 UTC |
      | null   | NULL, '2024-01-15 10:30:00 +00:00'                                    | 2024-01-15 10:30:00 UTC, NULL                     |

  @python_e2e
  Scenario: should download large result set with multiple chunks from table for timestamp_ltz
    Given Snowflake client is logged in
    And Table with TIMESTAMP_LTZ column exists with 50000 sequential timestamp values
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 UTC

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select timestamp_ltz using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_LTZ, ?::TIMESTAMP_LTZ" is executed with bound timestamp values
    Then Result should contain the bound timestamps

  @python_e2e
  Scenario: should select null timestamp_ltz using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_LTZ" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e
  Scenario: should insert timestamp_ltz using parameter binding
    Given Snowflake client is logged in
    And Table with TIMESTAMP_LTZ column exists
    When Timestamp values are bulk-inserted using multirow binding
    And Query "SELECT * FROM <table> ORDER BY col" is executed
    Then SELECT should return the same values in any order
