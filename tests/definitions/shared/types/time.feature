@python @core_not_needed
Feature: TIME type support
  # Snowflake TIME stores wallclock time in the form HH:MI:SS with optional fractional seconds.
  # Precision parameter: TIME(0) to TIME(9); default precision is 9 (nanoseconds).
  # Valid range: 00:00:00 to 23:59:59.999999999.
  # No timezone handling — all operations ignore time zones.
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-datetime#time

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast time values to appropriate type
    # Python: Values should be cast to 'datetime.time' type
    Given Snowflake client is logged in
    When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
    Then All values should be returned as appropriate type

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select time <values>
    Given Snowflake client is logged in
    When Query "SELECT <query_values>" is executed
    Then Result should contain times <expected_values>

    Examples:
      | values       | query_values                                         | expected_values              |
      | basic        | '10:30:00'::TIME, '14:45:30'::TIME, '23:59:59'::TIME | 10:30:00, 14:45:30, 23:59:59 |
      | midnight     | '00:00:00'::TIME                                     | 00:00:00                     |
      | microseconds | '10:30:00.123456'::TIME                              | 10:30:00.123456              |

  @python_e2e
  Scenario Outline: should handle time precision <scale>
    Given Snowflake client is logged in
    When Query "SELECT '10:30:00.123456789'::TIME(<scale>)" is executed
    Then Result should contain [<expected>]

    Examples:
      | scale | expected        |
      | 0     | 10:30:00        |
      | 3     | 10:30:00.123    |
      | 6     | 10:30:00.123456 |

  # Python's datetime.time supports microsecond precision (6 digits); nanoseconds are truncated.
  # Nanosecond-precision testing is handled by driver-specific tests where applicable.
  @python_not_needed
  Scenario: should preserve nanosecond precision for time
    Given Snowflake client is logged in
    When Query "SELECT '10:30:00.123456789'::TIME" is executed
    Then Result should contain [10:30:00.123456789]

  @python_e2e
  Scenario: should handle NULL values for time
    Given Snowflake client is logged in
    When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
    Then Result should contain [10:30:00, NULL, 23:59:59]

  @python_e2e
  Scenario: should download large result set with multiple chunks for time
    Given Snowflake client is logged in
    When Query "SELECT TIMEADD(millisecond, ROW_NUMBER() OVER (ORDER BY seq4()) - 1, '00:00:00'::TIME) as t FROM TABLE(GENERATOR(ROWCOUNT => 100000)) ORDER BY t" is executed
    Then Result should contain 100000 sequentially increasing time values from 00:00:00

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select <values> from table for time
    Given Snowflake client is logged in
    And Table with TIME column exists with values <insert_values>
    When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    Then Result should contain times <expected_values>

    Examples:
      | values       | insert_values                      | expected_values              |
      | basic        | '10:30:00', '14:45:30', '23:59:59' | 10:30:00, 14:45:30, 23:59:59 |
      | midnight     | '00:00:00', '12:00:00', '23:59:59' | 00:00:00, 12:00:00, 23:59:59 |
      | microseconds | '10:30:00', '10:30:00.123456'      | 10:30:00, 10:30:00.123456    |
      | null         | NULL, '10:30:00'                   | 10:30:00, NULL               |

  @python_e2e
  Scenario: should download large result set with multiple chunks from table for time
    Given Snowflake client is logged in
    And Table with TIME column exists with 100000 sequential time values starting from 00:00:00
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain 100000 sequentially increasing time values from 00:00:00

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select time using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIME, ?::TIME, ?::TIME" is executed with bound time values [10:30:00, 14:45:30, 23:59:59]
    Then Result should contain times [10:30:00, 14:45:30, 23:59:59]

  @python_e2e
  Scenario: should select null time using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIME" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e
  Scenario: should insert time using parameter binding
    Given Snowflake client is logged in
    And Table with TIME column exists
    When Time values [00:00:00, 10:30:00, 14:45:30, 23:59:59] are inserted using binding
    And Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain times [00:00:00, 10:30:00, 14:45:30, 23:59:59]

  @python_e2e
  Scenario: should insert time with fractional seconds using parameter binding
    Given Snowflake client is logged in
    And Table with TIME column exists
    When Time values [10:30:00.123456, 14:45:30.654321] are bulk-inserted using multirow binding
    And Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain times [10:30:00.123456, 14:45:30.654321]
