@python
Feature: VECTOR type support
  # Snowflake VECTOR type stores fixed-size arrays of numeric values.
  # Subtypes: INT (integer) and FLOAT (32-bit floating-point).
  # Values are returned as Python lists (list[int] or list[float]).
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
  Scenario: should select integer vector literals
    Given Snowflake client is logged in
    When Query "SELECT [1, 3, -5]::VECTOR(INT, 3), [40, 1234567]::VECTOR(INT, 2)" is executed
    Then Result should contain integer vectors [1, 3, -5] and [40, 1234567]

  @python_e2e
  Scenario: should select float vector literals
    Given Snowflake client is logged in
    When Query "SELECT [1.8, -3.4, 6.7, 0, 2.3]::VECTOR(FLOAT, 5)" is executed
    Then Result should contain float vector [1.8, -3.4, 6.7, 0, 2.3]

  # =========================================================================== #
  #                             NULL handling                                   #
  # =========================================================================== #

  @python_e2e
  Scenario: should handle NULL vector values from literals
    Given Snowflake client is logged in
    When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), NULL::VECTOR(INT, 3), NULL::VECTOR(FLOAT, 3)" is executed
    Then Result should contain [[1, 2, 3], NULL, NULL]

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
    And Table with VECTOR columns exists containing NULLs and values
    When Query "SELECT * FROM <table> ORDER BY id" is executed
    Then Result should contain both vector values and NULLs

  # =========================================================================== #
  #                       Multiple chunks downloading                           #
  # =========================================================================== #

  @python_e2e
  Scenario: should download vector data in multiple chunks
    Given Snowflake client is logged in
    When Query "SELECT [seq8(), seq8() * 2, seq8() * 3]::VECTOR(INT, 3) AS vec FROM TABLE(GENERATOR(ROWCOUNT => 20000)) v" is executed
    Then All 20000 rows should be fetched and each should be a non-null list value

  # =========================================================================== #
  #                         JSON result format                                  #
  # =========================================================================== #

  @python_e2e
  Scenario: should select vector with JSON result format
    Given Snowflake client is logged in
    And Session parameter PYTHON_CONNECTOR_QUERY_RESULT_FORMAT is set to JSON
    When Query "SELECT [1, 2, 3]::VECTOR(INT, 3), [1.5, 2.5, 3.5]::VECTOR(FLOAT, 3)" is executed
    Then Result should contain the expected integer and float vector values
