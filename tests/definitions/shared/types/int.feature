@python
Feature: INT type support

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast integer values to appropriate type for int and synonyms
    # Python: Values should be cast to 'int' type
    Given Snowflake client is logged in
    When Query "SELECT 0::<type>, 1000000::<type>, 9223372036854775807::<type>" is executed
    Then All values should be returned as appropriate type
    And No precision loss should occur

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should select integer literals for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT 0::<type>, 1::<type>, -1::<type>, 42::<type>" is executed
    Then Result should contain integers [0, 1, -1, 42]

  @python_e2e
  Scenario: should handle integer boundary values for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT -128::<type>, 127::<type>, 255::<type>" is executed
    Then Result should contain integers [-128, 127, 255]
    When Query "SELECT -32768::<type>, 32767::<type>, 65535::<type>" is executed
    Then Result should contain integers [-32768, 32767, 65535]
    When Query "SELECT -2147483648::<type>, 2147483647::<type>, 4294967295::<type>" is executed
    Then Result should contain integers [-2147483648, 2147483647, 4294967295]
    When Query "SELECT -9223372036854775808::<type>, 9223372036854775807::<type>" is executed
    Then Result should contain integers [-9223372036854775808, 9223372036854775807]

  @python_e2e
  Scenario: should handle large integer values for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT -99999999999999999999999999999999999999::<type>, 99999999999999999999999999999999999999::<type>" is executed
    Then Result should contain integers [-99999999999999999999999999999999999999, 99999999999999999999999999999999999999]

  @python_e2e
  Scenario: should handle NULL values for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT NULL::<type>, 42::<type>, NULL::<type>" is executed
    Then Result should contain [NULL, 42, NULL]

  @python_e2e
  Scenario: should download large result set with multiple chunks for int and synonyms
    Given Snowflake client is logged in
    When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 1000000)) v ORDER BY id" is executed
    Then Result should contain 1000000 sequentially numbered rows from 0 to 999999

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select integers from table for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with values [0, 1, -1, 100]
    When Query "SELECT * FROM int_table ORDER BY col" is executed
    Then Result should contain integers [-1, 0, 1, 100]

  @python_e2e
  Scenario: should select corner case values from table for int and synonyms
    Given Snowflake client is logged in
    And Table with columns (tinyint_col TINYINT, byteint_col BYTEINT, smallint_col SMALLINT, int_col INT, integer_col INTEGER, bigint_col BIGINT, int38_col INT) exists
    And Row with positive values (127, 255, 32767, 2147483647, 2147483647, 9223372036854775807, 99999999999999999999999999999999999999) is inserted
    And Row with negative values (-128, -1, -32768, -2147483648, -2147483648, -9223372036854775808, -99999999999999999999999999999999999999) is inserted
    And Row with zeroes and nulls (0, NULL, 0, NULL, 0, NULL, 0) is inserted
    When Query "SELECT * FROM corner_case_table" is executed
    Then Result should contain 3 rows with expected corner case values for all int type synonyms

  @python_e2e
  Scenario: should select large result set from table for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists with 1000000 sequential values
    When Query "SELECT * FROM <table> ORDER BY col" is executed
    Then Result should contain 1000000 sequentially numbered rows from 0 to 999999

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should insert integer using parameter binding for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists
    When Integer values [0, -2147483648, 2147483647, 9223372036854775807] are inserted using binding
    And Query "SELECT * FROM <table>" is executed
    Then Result should contain integers [0, -2147483648, 2147483647, 9223372036854775807]

  @python_e2e
  Scenario: should insert and select integers from table using parameter binding for int and synonyms
    Given Snowflake client is logged in
    And Table with <type> column exists
    When Integer values [0, 42, -2147483648, 9223372036854775807] are inserted using binding
    And Query "SELECT * FROM <table>" is executed
    Then Result should contain integers [0, 42, -2147483648, 9223372036854775807]
