@python @odbc @jdbc @core_not_needed
Feature: INT type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should cast integer values to appropriate type for int and synonyms
    # Python: Values should be cast to 'int' type
    Given Snowflake client is logged in
    When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
    Then All values should be returned as appropriate type
    And No precision loss should occur

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario Outline: should select integer <values> for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT <query_values>" is executed
    Then Result should contain integers <expected_values>

    Examples:
      | values     | query_values                                                        | expected_values                              |
      | zero       | 0::<type>                                                           | 0                                            |
      | tinyint    | -128::<type>, 127::<type>, 255::<type>                              | -128, 127, 255                               |
      | smallint   | -32768::<type>, 32767::<type>, 65535::<type>                        | -32768, 32767, 65535                         |
      | int        | -2147483648::<type>, 2147483647::<type>, 4294967295::<type>         | -2147483648, 2147483647, 4294967295          |
      | bigint     | -9223372036854775808::<type>, 9223372036854775807::<type>           | -9223372036854775808, 9223372036854775807    |

  @python_e2e @jdbc_e2e
  Scenario: should handle large integer values for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT -99999999999999999999999999999999999999::<type>, 99999999999999999999999999999999999999::<type>" is executed
    Then Result should contain integers [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]

  @python_e2e @jdbc_e2e
  Scenario: should handle NULL values for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
    Then Result should contain [NULL, 42, NULL]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should download large result set with multiple chunks for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v ORDER BY id" is executed
    Then Result should contain 50000 sequentially numbered rows from 0 to 49999

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario Outline: should select <values> from table for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with values <insert_values>
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain integers <expected_values>

    Examples:
      | values      | insert_values                                                             | expected_values                                                               |
      | positive    | 0, 1, 127, 255, 32767, 65535, 2147483647, 4294967295, 9223372036854775807 | 0, 1, 127, 255, 32767, 65535, 2147483647, 4294967295, 9223372036854775807     |
      | negative    | -1, -128, -32768, -2147483648, -9223372036854775808                       | -9223372036854775808, -2147483648, -32768, -128, -1                           |
      | null        | 0, NULL, 42                                                               | 0, 42, NULL                                                                   |

  @python_e2e @jdbc_e2e
  Scenario: should select large integer values from table for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with values [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain integers [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]

  @python_e2e @odbc_e2e @jdbc_e2e
  Scenario: should handle server-side Arrow memory optimization for int columns on multiple chunks
    Given Snowflake client is logged in
    And Table with four INT columns exists
    And Each column contains values of different magnitudes (50000 rows to span multiple Arrow chunks)
      | Column    | Values          | Arrow Type |
      | col_int8  | -128 to 127     | Int8       |
      | col_int16 | -32768 to 32767 | Int16      |
      | col_int32 | -2B to 2B       | Int32      |
      | col_int64 | -9Q to 9Q       | Int64      |
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain 50000 rows
    And All values should be equal to expected data

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e @jdbc_e2e
  Scenario: should insert integer using parameter binding for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists
    When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
    And Query "SELECT * FROM <table>" is executed
    Then Result should contain integers [0, -2147483648, 2147483647, 9223372036854775807]

  @python_e2e
  Scenario: should insert and select integers from table using batch parameter binding for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists
    When Integer values [0, 42, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
    And Query "SELECT * FROM <table>" is executed
    Then Result should contain integers [0, 42, -2147483648, 2147483647, 9223372036854775807]
