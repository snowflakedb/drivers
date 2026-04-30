@python @core_not_needed
Feature: VECTOR type support
  # Snowflake VECTOR type stores fixed-size arrays of numeric values.
  # Subtypes: INT (integer) and FLOAT (32-bit floating-point).
  # Maximum dimension: 4096.
  # Reference: https://docs.snowflake.com/en/sql-reference/data-types-vector

  # =========================================================================== #
  #                               Type casting                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should cast vector values to appropriate type
    # Python: INT vectors should be list[int], FLOAT vectors should be list[float]
    Given Snowflake client is logged in
    When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
    Then All values should be returned as appropriate type

  # =========================================================================== #
  #                     SELECT with literals (no tables)                        #
  # =========================================================================== #

  @python_e2e
  Scenario Outline: should select <subtype> vector literal
    Given Snowflake client is logged in
    When Query "SELECT <expected_value>::VECTOR(<vec_type>, ...)" is executed
    Then Result should contain <subtype> vector <expected_value>

    Examples:
      | subtype   | vec_type | expected_value              |
      | INT-3d    | INT      | [1, 3, -5]                  |
      | INT-2d    | INT      | [40, 1234567]               |
      | FLOAT-5d  | FLOAT    | [1.8, -3.4, 6.7, 0.0, 2.3] |

  @python_e2e
  Scenario: should handle NULL vector values
    Given Snowflake client is logged in
    When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
    Then Result should contain [[1, 2, 3], NULL, NULL]

  @python_e2e
  Scenario: should select max-dimension vector
    Given Snowflake client is logged in
    When Query selecting 4096-element float vector is executed
    Then Result should be a valid 4096-element float vector

  # =========================================================================== #
  #                           Table operations                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should select vector values from table
    Given Snowflake client is logged in
    And Table with VECTOR(INT, 3) and VECTOR(FLOAT, 5) columns exists with values
    When Query "SELECT * FROM <table> ORDER BY id" is executed
    Then Result should contain the expected integer and float vector values

  @python_e2e
  Scenario: should handle NULL vector values from table
    Given Snowflake client is logged in
    And Table with VECTOR columns exist containing NULLs and values
    When Query "SELECT * FROM <table> ORDER BY id" is executed
    Then Result should contain both vector values and NULLs

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e
  Scenario: should download vector data in multiple chunks
    Given Snowflake client is logged in
    When Query generating 20000 integer vectors is executed
    Then All 20000 rows should be fetched with valid 3-element integer vectors
