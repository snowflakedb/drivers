@python @odbc
Feature: ARRAY type support
  # Snowflake ARRAY is a semi-structured data type that stores ordered lists of values.
  # Values can be any Snowflake type including nested ARRAYs and OBJECTs.
  # Constructed via ARRAY_CONSTRUCT(val1, val2, ...).
  # Returned as a JSON string representation (e.g. '[1, 2, 3]').
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should cast array values to appropriate type
    # Python: Values should be cast to 'list' type (or 'str' depending on driver)
    # ODBC: Values are returned as SQL_C_CHAR (JSON string)
    Given Snowflake client is logged in
    When Query "SELECT ARRAY_CONSTRUCT(1, 2, 3)::ARRAY" is executed
    Then Value should be returned as appropriate type
    And Value should be an array containing elements [1, 2, 3]

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select hardcoded array literals
    Given Snowflake client is logged in
    When Query "SELECT ARRAY_CONSTRUCT('a', 'b', 'c')" is executed
    Then Result should contain an array with 3 elements
    And Array values should be ['a', 'b', 'c']

  @python_e2e @odbc_e2e
  Scenario: should select array corner case values from literals
    # Corner cases:
    #   - Empty array: ARRAY_CONSTRUCT()
    #   - Single element array: ARRAY_CONSTRUCT(42)
    #   - Nested array: ARRAY_CONSTRUCT(ARRAY_CONSTRUCT(1, 2), ARRAY_CONSTRUCT(3, 4))
    #   - Mixed types: ARRAY_CONSTRUCT(1, 'two', TRUE)
    #   - Array with object: ARRAY_CONSTRUCT(OBJECT_CONSTRUCT('key', 'value'))
    #   - NULL::ARRAY
    Given Snowflake client is logged in
    When Queries selecting corner case array literals are executed
    Then Results should contain expected corner case array values

  # =========================================================================== #
  #                     SELECT FROM TABLE (Happy path, Corner cases)            #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select array values from table
    Given Snowflake client is logged in
    And A temporary table with VARIANT column is created
    And The table is populated with array values
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted array values

  @python_e2e @odbc_e2e
  Scenario: should select array corner case values from table
    Given Snowflake client is logged in
    And A temporary table with VARIANT column is created
    And The table is populated with corner case array values
    # Corner cases:
    #   - Empty array
    #   - Nested arrays
    #   - Array with mixed types
    #   - NULL row
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted corner case array values

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select array using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON(?)" is executed with bound JSON array string
    Then Result should contain a valid array
    When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e
  Scenario: should insert array using parameter binding
    Given Snowflake client is logged in
    And A temporary table with VARIANT column is created
    When JSON array string is inserted using parameter binding via PARSE_JSON
    And Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted array

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should download array data in multiple chunks
    # ~10000 rows of array data to ensure multiple chunks
    Given Snowflake client is logged in
    When Query selecting 10000 ARRAY_CONSTRUCT rows from GENERATOR is executed
    Then there are 10000 rows returned
    And All returned values should be valid array representations
