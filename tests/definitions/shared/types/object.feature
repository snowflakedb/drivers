@python @odbc
Feature: OBJECT type support
  # Snowflake OBJECT is a semi-structured data type that stores key-value pairs.
  # Keys are always strings; values can be any Snowflake type including nested OBJECTs.
  # Constructed via OBJECT_CONSTRUCT('key1', val1, 'key2', val2, ...).
  # Returned as a JSON string representation (e.g. '{"key":"value"}').
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should cast object values to appropriate type
    # Python: Values should be cast to 'dict' type (or 'str' depending on driver)
    # ODBC: Values are returned as SQL_C_CHAR (JSON string)
    Given Snowflake client is logged in
    When Query "SELECT OBJECT_CONSTRUCT('name', 'Alice', 'age', 30)::OBJECT" is executed
    Then Value should be returned as appropriate type
    And Value should contain key 'name' with value 'Alice' and key 'age' with value 30

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select hardcoded object literals
    Given Snowflake client is logged in
    When Query "SELECT OBJECT_CONSTRUCT('key1', 'value1', 'key2', 42)" is executed
    Then Result should contain an object with keys [key1, key2]
    And Object values should be key1='value1' and key2=42

  @python_e2e @odbc_e2e
  Scenario: should select object corner case values from literals
    # Corner cases:
    #   - Empty object: OBJECT_CONSTRUCT()
    #   - Object with NULL value: OBJECT_CONSTRUCT('key', NULL) — key may be omitted
    #   - Nested object: OBJECT_CONSTRUCT('outer', OBJECT_CONSTRUCT('inner', 'value'))
    #   - Object with boolean: OBJECT_CONSTRUCT('flag', TRUE)
    #   - Object with numeric types: OBJECT_CONSTRUCT('int', 1, 'float', 1.5)
    #   - Object with unicode key/value: OBJECT_CONSTRUCT('日本語', 'テスト')
    #   - NULL::OBJECT
    Given Snowflake client is logged in
    When Queries selecting corner case object literals are executed
    Then Results should contain expected corner case object values

  # =========================================================================== #
  #                     SELECT FROM TABLE (Happy path, Corner cases)            #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select object values from table
    Given Snowflake client is logged in
    And A temporary table with VARIANT column is created
    And The table is populated with object values
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted object values

  @python_e2e @odbc_e2e
  Scenario: should select object corner case values from table
    Given Snowflake client is logged in
    And A temporary table with VARIANT column is created
    And The table is populated with corner case object values
    # Corner cases:
    #   - Empty object
    #   - Nested objects
    #   - Object with various value types (string, number, boolean, null, array)
    #   - NULL row
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted corner case object values

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select object using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string
    Then Result should contain a valid object
    When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e
  Scenario: should insert object using parameter binding
    Given Snowflake client is logged in
    And A temporary table with VARIANT column is created
    When JSON string is inserted using parameter binding via PARSE_JSON
    And Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted object

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should download object data in multiple chunks
    # ~10000 rows of object data to ensure multiple chunks
    Given Snowflake client is logged in
    When Query selecting 10000 OBJECT_CONSTRUCT rows from GENERATOR is executed
    Then there are 10000 rows returned
    And All returned values should be valid object representations
