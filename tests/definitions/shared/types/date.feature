@jdbc @core_not_needed
Feature: DATE type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @jdbc_e2e
  Scenario: should cast date values to appropriate type
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
    Then All values should be returned as DATE type
    And No precision loss should occur

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @jdbc_e2e
  Scenario: should select date literals
    Given Snowflake client is logged in
    When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
    Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]

  @jdbc_e2e
  Scenario: should select epoch and pre-epoch dates
    Given Snowflake client is logged in
    When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
    Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]

  @jdbc_e2e
  Scenario: should select historical dates
    Given Snowflake client is logged in
    When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE" is executed
    Then Result should contain dates [0001-01-01, 1582-10-15]

  @jdbc_e2e
  Scenario: should handle NULL values for date
    Given Snowflake client is logged in
    When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
    Then Result should contain [NULL, 2024-01-15, NULL]

  @jdbc_e2e
  Scenario: should download large result set for date
    Given Snowflake client is logged in
    When Query "SELECT DATEADD(day, seq4(), '1970-01-01'::DATE) as d FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY d" is executed
    Then Result should contain 50000 rows with sequential dates starting from 1970-01-01

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @jdbc_e2e
  Scenario: should select dates from table
    Given Snowflake client is logged in
    And Table with DATE column exists with values ['2024-01-15', '1970-01-01', '1999-12-31']
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]

  @jdbc_e2e
  Scenario: should select dates with NULL from table
    Given Snowflake client is logged in
    And Table with DATE column exists with values ['2024-01-15', NULL, '1999-12-31']
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain [1999-12-31, 2024-01-15, NULL]

  @jdbc_e2e
  Scenario: should select historical dates from table
    Given Snowflake client is logged in
    And Table with DATE column exists with values ['0001-01-01', '0100-03-01', '1582-10-15']
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain dates [0001-01-01, 0100-03-01, 1582-10-15]

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @jdbc_e2e
  Scenario: should select date using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::DATE, ?::DATE, ?::DATE" is executed with bound date values [2024-01-15, 1970-01-01, 1999-12-31]
    Then Result should contain [2024-01-15, 1970-01-01, 1999-12-31]

  @jdbc_e2e
  Scenario: should select null date using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT ?::DATE" is executed with bound NULL value
    Then Result should contain [NULL]

  @jdbc_e2e
  Scenario: should insert date using parameter binding
    Given Snowflake client is logged in
    And Table with DATE column exists
    When Date values [2024-01-15, 1970-01-01, 1999-12-31] are inserted using setDate binding
    And Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain dates [1970-01-01, 1999-12-31, 2024-01-15]
