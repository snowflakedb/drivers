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
