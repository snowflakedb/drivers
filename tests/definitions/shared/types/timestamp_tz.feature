@python
Feature: TIMESTAMP_TZ type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast timestamp_tz values to appropriate type
    # Python: Values should be cast to 'datetime' type with tzinfo set
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ" is executed
    Then All values should be returned as appropriate type
    And Values should have timezone info

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select timestamp_tz <values>
    Given Snowflake client is logged in
    When Query "SELECT <query_values>" is executed
    Then Result should contain timestamps <expected_values>
    And Values should have timezone info

    Examples:
      | values       | query_values                                                                                      | expected_values                                         |
      | basic        | '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, '2024-06-20 14:45:30 -08:00'::TIMESTAMP_TZ         | 2024-01-15 10:30:00 +05:00, 2024-06-20 14:45:30 -08:00 |
      | epoch        | '1970-01-01 00:00:00 +00:00'::TIMESTAMP_TZ                                                       | 1970-01-01 00:00:00 +00:00                              |
      | microseconds | '2024-01-15 10:30:00.123456 +05:00'::TIMESTAMP_TZ                                                | 2024-01-15 10:30:00.123456 +05:00                       |

  @python_e2e
  Scenario: should preserve timezone offset for timestamp_tz
    # TIMESTAMP_TZ preserves the original offset — unlike LTZ which converts to session TZ
    # Includes fractional offsets (+05:30, +04:30, -02:30) found in real-world timezones
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ, '2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ, '2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ, '2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ, '2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ" is executed
    Then Result should preserve offsets [+05:30, -08:00, +00:00, +04:30, -02:30]

  @python_e2e
  Scenario Outline: should select edge date timestamp_tz <values>
    Given Snowflake client is logged in
    When Query "SELECT <query_values>" is executed
    Then Result should contain timestamps <expected_values>
    And Values should have timezone info

    Examples:
      | values     | query_values                                                   | expected_values                  |
      | year 9999  | '9999-12-31 23:59:59 +00:00'::TIMESTAMP_TZ                    | 9999-12-31 23:59:59 +00:00      |
      | year 1900  | '1900-01-01 00:00:00 +00:00'::TIMESTAMP_TZ                    | 1900-01-01 00:00:00 +00:00      |
      | pre-epoch  | '1960-06-15 12:00:00 +05:00'::TIMESTAMP_TZ                    | 1960-06-15 12:00:00 +05:00      |

  @python_e2e
  Scenario: should handle NULL values for timestamp_tz
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ" is executed
    Then Result should contain [2024-01-15 10:30:00 +05:00, NULL]

  @python_e2e
  Scenario: should download large result set with multiple chunks for timestamp_tz
    Given Snowflake client is logged in
    When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
    Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select <values> from table for timestamp_tz
    Given Snowflake client is logged in
    And Table with TIMESTAMP_TZ column exists with values <insert_values>
    When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    Then Result should contain timestamps <expected_values>
    And Values should have timezone info

    Examples:
      | values       | insert_values                                                               | expected_values                                                     |
      | basic        | '2024-01-15 10:30:00 +05:00', '2024-06-20 14:45:30 -08:00'                | 2024-01-15 10:30:00 +05:00, 2024-06-20 14:45:30 -08:00             |
      | epoch        | '1970-01-01 00:00:00 +00:00', '2024-01-15 10:30:00 +05:00'                | 1970-01-01 00:00:00 +00:00, 2024-01-15 10:30:00 +05:00             |
      | microseconds | '2024-01-15 10:30:00 +05:00', '2024-01-15 10:30:00.123456 +05:00'          | 2024-01-15 10:30:00 +05:00, 2024-01-15 10:30:00.123456 +05:00     |
      | null         | NULL, '2024-01-15 10:30:00 +05:00'                                         | 2024-01-15 10:30:00 +05:00, NULL                                    |

  @python_e2e
  Scenario: should download large result set with multiple chunks from table for timestamp_tz
    Given Snowflake client is logged in
    And Table with TIMESTAMP_TZ column exists with 50000 sequential timestamp values
    When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select timestamp_tz using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ" is executed with bound timestamp values
    Then Result should contain the bound timestamps
    And Values should have timezone info

  @python_e2e
  Scenario: should select null timestamp_tz using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_TZ" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e
  Scenario: should insert timestamp_tz using parameter binding
    Given Snowflake client is logged in
    And Table with TIMESTAMP_TZ column exists
    When Timestamp values are bulk-inserted using multirow binding
    And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    Then SELECT should return the same values in any order
