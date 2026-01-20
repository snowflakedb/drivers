@python @numpy
Feature: DECFLOAT type NumPy support (Python-specific)

  @python_e2e
  Scenario: should cast decfloat values to numpy float64
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT 1.234::DECFLOAT, 123.456::DECFLOAT, -789.012::DECFLOAT" is executed
    Then All values should be returned as numpy.float64 type
    And Values should match approximately [1.234, 123.456, -789.012] within float64 precision

  @python_e2e
  Scenario: numpy handles extreme exponents within float64 range
    Given Snowflake client is logged in with NumPy mode enabled
    When Query with exponents within float64 range is executed
    Then Values should be numpy.float64
    And Values should be approximately correct

  @python_e2e
  Scenario: numpy overflows extreme exponents beyond float64 range
    Given Snowflake client is logged in with NumPy mode enabled
    When Query with exponents exceeding float64 range is executed
    Then e16384 exceeds float64 max (~e308) and becomes infinity
    And e-16383 is below float64 min (~e-308) and becomes 0

