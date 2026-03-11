Feature: TIMESTAMP_NTZ type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  Scenario: should cast timestamp_ntz values to appropriate type
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ" is executed
    Then All values should be returned as appropriate type
    And Values should not have timezone info

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  Scenario Outline: should select timestamp_ntz <values>
    Given Snowflake client is logged in
    When Query "SELECT <query_values>" is executed
    Then Result should contain timestamps <expected_values>
    And Values should not have timezone info

    Examples:
      | values       | query_values                                                                      | expected_values                             |
      | basic        | '2024-01-15 10:30:00'::TIMESTAMP_NTZ, '2024-06-20 14:45:30'::TIMESTAMP_NTZ       | 2024-01-15 10:30:00, 2024-06-20 14:45:30   |
      | epoch        | '1970-01-01 00:00:00'::TIMESTAMP_NTZ                                              | 1970-01-01 00:00:00                         |
      | microseconds | '2024-01-15 10:30:00.123456'::TIMESTAMP_NTZ                                       | 2024-01-15 10:30:00.123456                  |

  Scenario: should handle NULL values for timestamp_ntz
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
    Then Result should contain [2024-01-15 10:30:00, NULL]

  Scenario: should download large result set with multiple chunks for timestamp_ntz
    Given Snowflake client is logged in
    When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, '2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
    Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  Scenario Outline: should select <values> from table for timestamp_ntz
    Given Snowflake client is logged in
    And Table with TIMESTAMP_NTZ column exists with values <insert_values>
    When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    Then Result should contain timestamps <expected_values>
    And Values should not have timezone info

    Examples:
      | values       | insert_values                                         | expected_values                               |
      | basic        | '2024-01-15 10:30:00', '2024-06-20 14:45:30'          | 2024-01-15 10:30:00, 2024-06-20 14:45:30     |
      | epoch        | '1970-01-01 00:00:00', '2024-01-15 10:30:00'          | 1970-01-01 00:00:00, 2024-01-15 10:30:00     |
      | microseconds | '2024-01-15 10:30:00', '2024-01-15 10:30:00.123456'   | 2024-01-15 10:30:00, 2024-01-15 10:30:00.123456 |
      | null         | NULL, '2024-01-15 10:30:00'                           | 2024-01-15 10:30:00, NULL                    |

  Scenario: should download large result set with multiple chunks from table for timestamp_ntz
    Given Snowflake client is logged in
    And Table with TIMESTAMP_NTZ column exists with 50000 sequential timestamp values
    When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  Scenario: should select timestamp_ntz using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ" is executed with bound timestamp values
    Then Result should contain [2024-01-15 10:30:00, 2024-06-20 14:45:30]
    And Values should not have timezone info

  Scenario: should return NULL when selecting timestamp_ntz using parameter binding with NULL value
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound NULL value
    Then Result should contain [NULL]

  Scenario Outline: should store UTC equivalent when binding timezone-aware datetime to timestamp_ntz
    Given Snowflake client is logged in
    When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound aware datetime <input>
    Then Result should contain [<expected>]
    And Values should not have timezone info

    Examples:
      | input                     | expected              |
      | 2024-01-15 10:30:00+00:00 | 2024-01-15 10:30:00   |
      | 2024-01-15 12:30:00+02:00 | 2024-01-15 10:30:00   |
      | 2024-01-15 10:30:00-05:00 | 2024-01-15 15:30:00   |

  Scenario: should insert timestamp_ntz using parameter binding
    Given Snowflake client is logged in
    And Table with TIMESTAMP_NTZ column exists
    When Timestamp values are bulk-inserted using multirow binding
    And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
    Then SELECT should return the inserted values in ascending order

  # =========================================================================== #
  #                            Type mapping aliases                             #
  # =========================================================================== #

  Scenario Outline: should return naive datetime for <type_name> alias when session mapping is TIMESTAMP_NTZ
    Given Snowflake client is logged in
    And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_NTZ
    When Query "SELECT '2024-01-15 10:30:00'::<type_name>" is executed
    Then All values should be returned as appropriate type
    And Values should not have timezone info

    Examples:
      | type_name |
      | TIMESTAMP |
      | DATETIME  |

  Scenario: should return aware datetime for TIMESTAMP alias when session mapping is TIMESTAMP_LTZ
    Given Snowflake client is logged in
    And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_LTZ
    When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP" is executed
    Then All values should be returned as appropriate type
    And Values should have timezone info

  Scenario: should preserve nanosecond precision for timestamp_ntz
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ" is executed
    Then Result should contain [2024-01-15 10:30:00.123456789]
