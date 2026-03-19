@python @numpy
Feature: FLOAT type NumPy support (Python-specific)

  @python_e2e
  Scenario: should cast float values to numpy float64 for float and synonyms
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT 0.0::<type>, 123.456::<type>, -789.012::<type>, 1.23e10::<type>" is executed
    Then All values should be returned as numpy.float64 type
    And Values should match expected floats [0.0, 123.456, -789.012, 1.23e10]

  @python_e2e
  Scenario: should handle special float values with numpy for float and synonyms
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
    Then All values should be returned as numpy.float64 type
    And Result should contain [NaN, positive_infinity, negative_infinity]


