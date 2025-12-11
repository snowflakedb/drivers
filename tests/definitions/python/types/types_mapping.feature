@python
Feature: Types mapping

  @python
  Scenario: should cast NUMBER to integer when scale is 0
    Given Snowflake client is logged in
    When Query selecting values of NUMBER, DECIMAL, DEC, NUMERIC with scale 0 is executed
    Then All returned values should be cast to integers
    And All returned values should be equal to the expected literals

  @python
  Scenario: should cast NUMBER to Decimal when scale is nonzero
    Given Snowflake client is logged in
    When Query selecting values of NUMBER, DECIMAL, DEC, NUMERIC with scale > 0 is executed
    Then All returned values should be cast to Decimal
    And All returned values should be equal to the expected literals

  @python
  Scenario: should cast INT and its synonyms to integer
    Given Snowflake client is logged in
    When Query selecting values of INT, INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT is executed
    Then All returned values should be cast to integers
    And All returned values should be equal to the expected literals

  @python
  Scenario: should cast FLOAT and its synonyms to float
    Given Snowflake client is logged in
    When Query selecting values of FLOAT, FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL is executed
    Then All returned values should be cast to floats
    And All returned values should be equal to the expected literals

  @python
  Scenario: should cast FLOAT subnormal value 1e-324 to zero
    Given Snowflake client is logged in
    When Query selecting subnormal float value 1e-324 is executed
    Then The returned value should be cast to float
    And The returned value should be equal to 0.0

  @python
  Scenario: should cast DECFLOAT to Decimal
    Given Snowflake client is logged in
    When Query selecting values of DECFLOAT is executed
    Then All returned values should be cast to Decimal
    And All returned values should be equal to the expected literals
