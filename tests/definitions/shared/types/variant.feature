@python
Feature: VARIANT type support
  # Snowflake VARIANT is a semi-structured data type that can store any JSON value:
  # objects, arrays, strings, numbers, booleans, and null.
  # Values are returned as JSON-serialized strings.
  # Maximum size: 16 MB (compressed)
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-semistructured

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast variant values to appropriate type
    # Python: Values should be cast to 'str' type (JSON-serialized)
    Given Snowflake client is logged in
    When Query selecting a JSON object as VARIANT is executed
    Then All values should be returned as string type
    And Value should be a valid JSON object

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should select variant literals
    Given Snowflake client is logged in
    When Query selecting JSON object and array as VARIANT literals is executed
    Then Result should contain valid JSON object and array values

  @python_e2e
  Scenario: should select variant corner case values
    # Corner cases for VARIANT:
    #   - Empty object
    #   - Empty array
    #   - JSON null
    #   - Nested object
    #   - Mixed array with different types
    #   - Boolean values: true, false
    #   - Numeric values: integer, float, negative
    #   - String value in variant
    Given Snowflake client is logged in
    When Query selecting corner case VARIANT literals is executed
    Then Result should contain expected corner case VARIANT values

  @python_e2e
  Scenario: should handle NULL values from literals
    Given Snowflake client is logged in
    When Query selecting VARIANT values with SQL NULL is executed
    Then Result should contain JSON values and SQL NULLs in expected positions

  # =========================================================================== #
  #                             Table operations                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select variant values from table
    Given Snowflake client is logged in
    And Table with VARIANT column exists
    And VARIANT rows are inserted with PARSE_JSON values
    When Query selecting all rows from VARIANT table is executed
    Then Result should contain the inserted VARIANT values as JSON strings

  @python_e2e
  Scenario: should handle NULL values from table
    Given Snowflake client is logged in
    And Table with VARIANT column exists
    And VARIANT rows including NULLs are inserted
    When Query selecting all rows from VARIANT table is executed
    Then Result should contain NULL and non-NULL VARIANT values in any order

  # =========================================================================== #
  #                            Parameter binding                                #
  # =========================================================================== #

  @python_e2e
  Scenario: should select variant using parameter binding with PARSE_JSON
    Given Snowflake client is logged in
    When Query with PARSE_JSON binding is executed with a JSON string parameter
    Then Result should contain the expected JSON value
    When Query with PARSE_JSON binding is executed with NULL parameter
    Then Result should contain NULL

  @python_e2e
  Scenario: should insert variant using parameter binding
    Given Snowflake client is logged in
    And Table with VARIANT column exists
    When VARIANT values are inserted using parameter binding with PARSE_JSON
    Then SELECT should return the same VARIANT values

  # =========================================================================== #
  #                          Multiple chunks downloading                        #
  # =========================================================================== #

  @python_e2e
  Scenario: should download large result set with multiple chunks from table
    Given Snowflake client is logged in
    And Table with VARIANT column exists with 1000000 generated VARIANT values
    When Query selecting all rows from VARIANT table is executed
    Then Result should contain 1000000 VARIANT values
