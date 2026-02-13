@python @numpy
Feature: BOOLEAN type NumPy support (Python-specific)

  @python_e2e
  Scenario: should cast boolean to numpy bool type
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
    Then All values should be returned as numpy.bool_ type
    And Values should be [TRUE, FALSE, TRUE, FALSE]
