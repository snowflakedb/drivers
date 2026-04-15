@odbc
Feature: Semi-structured type (VARIANT/OBJECT/ARRAY) handling
  # Snowflake semi-structured types: VARIANT, OBJECT, ARRAY
  # Internal representation varies by driver and result format (JSON string, parsed
  # objects, or native Arrow vectors). These scenarios test logical correctness
  # without prescribing the return format.
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should cast semi-structured values to appropriate type
    # ODBC: Values are reported as SQL_VARCHAR
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON('{\"a\":1}'), ARRAY_CONSTRUCT(1,2,3), OBJECT_CONSTRUCT('key','val')" is executed
    Then All values should be returned as appropriate type

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should select semi-structured literals
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON('{\"key\":\"value\"}'), ARRAY_CONSTRUCT(10, 20, 30), OBJECT_CONSTRUCT('a', 1, 'b', 2)" is executed
    Then Result should contain the expected values for VARIANT, ARRAY, and OBJECT columns

  @odbc_e2e
  Scenario: should select deeply nested semi-structured literals
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON('{\"a\":{\"b\":[1,2,{\"c\":true}]}}')" is executed
    Then Result should contain the expected nested value

  # =========================================================================== #
  #                             NULL handling                                   #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should handle NULL semi-structured values from literals
    Given Snowflake client is logged in
    When Query "SELECT NULL::VARIANT, NULL::OBJECT, NULL::ARRAY" is executed
    Then All columns should return null indicators

  # =========================================================================== #
  #                           Table operations                                  #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should select semi-structured values from table
    Given Snowflake client is logged in
    And Table with VARIANT, OBJECT, and ARRAY columns exists with JSON values
    When Query "SELECT * FROM <table>" is executed
    Then Data should contain the expected semi-structured values

  @odbc_e2e
  Scenario: should handle NULL semi-structured values from table
    Given Snowflake client is logged in
    And Table with VARIANT column exists containing NULLs and values
    When Query "SELECT * FROM <table>" is executed
    Then Result should contain [NULL, {"a":1}, NULL]

  # =========================================================================== #
  #                         Empty JSON containers                               #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should handle empty JSON containers
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON('{}'), ARRAY_CONSTRUCT(), OBJECT_CONSTRUCT()" is executed
    Then Each column should return a valid empty container

  @odbc_e2e
  Scenario: should handle empty JSON array literal
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON('[]')" is executed
    Then Result should be an empty JSON array

  @odbc_e2e
  Scenario: should round-trip empty JSON containers through a table
    Given Snowflake client is logged in
    And Table with VARIANT, OBJECT, and ARRAY columns exists with empty containers
    When Query "SELECT * FROM <table>" is executed
    Then All columns should return valid empty containers

  # =========================================================================== #
  #                       JSON with unicode content                             #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should handle JSON with unicode content
    Given Snowflake client is logged in
    When Query returning JSON with unicode characters is executed
    Then Result should preserve the unicode characters

  @odbc_e2e
  Scenario: should handle JSON with unicode in keys
    Given Snowflake client is logged in
    When Query returning JSON with unicode characters in keys is executed
    Then Result should preserve unicode keys and their associated values

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should download semi-structured data in multiple chunks
    Given Snowflake client is logged in
    When Query "SELECT OBJECT_CONSTRUCT('id', seq8()) AS obj FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v ORDER BY 1" is executed
    Then All 20000 rows should be fetched and each should contain a value with "id" key

  # =========================================================================== #
  #                           Parameter binding                                 #
  # =========================================================================== #

  @odbc_e2e
  Scenario: should select variant using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON(?)" is executed with bound JSON string '{"bound":true}'
    Then Result should contain a value with "bound" key

  @odbc_e2e
  Scenario: should select NULL variant using parameter binding
    Given Snowflake client is logged in
    When Query "SELECT PARSE_JSON(?)" is executed with bound NULL value
    Then Result should be NULL

  @odbc_e2e
  Scenario: should insert variant using parameter binding
    Given Snowflake client is logged in
    And Table with VARIANT column exists
    When JSON values are inserted using parameter binding via PARSE_JSON(?)
    Then SELECT should return the inserted JSON values
