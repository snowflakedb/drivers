@python @odbc
Feature: MAP type support
  # Snowflake MAP is a semi-structured data type that stores key-value pairs
  # with typed keys and typed values, unlike OBJECT where keys are always strings.
  # Created by casting OBJECT to MAP or using MAP-typed columns.
  # Returned as a JSON string representation (e.g. '{"key1":"value1"}').
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should cast map values to appropriate type
    # Python: Values should be cast to 'dict' type (or 'str' depending on driver)
    # ODBC: Values are returned as SQL_C_CHAR (JSON string)
    Given Snowflake client is logged in
    When Query selecting a MAP(VARCHAR, VARCHAR) value is executed
    Then Value should be returned as appropriate type
    And Value should be a map containing key 'x' with value '1' and key 'y' with value '2'

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select hardcoded map literals
    Given Snowflake client is logged in
    When Query selecting a MAP(VARCHAR, INTEGER) value with keys [a, b] is executed
    Then Result should contain a map with 2 entries
    And Map values should be a=1 and b=2

  @python_e2e @odbc_e2e
  Scenario: should select map corner case values from literals
    # Corner cases:
    #   - Empty map
    #   - Single entry map
    #   - Map with NULL value
    #   - NULL::MAP(VARCHAR, VARCHAR)
    Given Snowflake client is logged in
    When Queries selecting corner case map literals are executed
    Then Results should contain expected corner case map values

  # =========================================================================== #
  #                     SELECT FROM TABLE (Happy path, Corner cases)            #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should select map values from table
    Given Snowflake client is logged in
    And A temporary table with MAP column is created
    And The table is populated with map values
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted map values

  @python_e2e @odbc_e2e
  Scenario: should select map corner case values from table
    Given Snowflake client is logged in
    And A temporary table with MAP column is created
    And The table is populated with corner case map values
    # Corner cases:
    #   - Empty map
    #   - Single entry map
    #   - NULL row
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted corner case map values

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select map using parameter binding
    Given Snowflake client is logged in
    When Query selecting PARSE_JSON with bound JSON map string is executed
    Then Result should contain a valid map
    When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
    Then Result should contain [NULL]

  @python_e2e
  Scenario: should insert map using parameter binding
    Given Snowflake client is logged in
    And A temporary table with MAP column is created
    When JSON map string is inserted using parameter binding
    And Query "SELECT * FROM <table>" is executed
    Then Result should contain the inserted map

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e @odbc_e2e
  Scenario: should download map data in multiple chunks
    # ~10000 rows of map data to ensure multiple chunks
    Given Snowflake client is logged in
    When Query selecting 10000 MAP rows from GENERATOR is executed
    Then there are 10000 rows returned
    And All returned values should be valid map representations
