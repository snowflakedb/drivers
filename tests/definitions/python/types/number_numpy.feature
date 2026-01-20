@python @numpy
Feature: NUMBER type NumPy support (Python-specific)

  @python_e2e
  Scenario: should cast number scale0 to numpy int64
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT 0::NUMBER(10,0), 123::NUMBER(10,0), -456::NUMBER(10,0), 999999::NUMBER(10,0)" is executed
    Then All values should be returned as numpy.int64 type
    And Values should match exactly [0, 123, -456, 999999]

  @python_e2e
  Scenario: should cast number scale3 to numpy float64
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT 0.000::NUMBER(15,3), 123.456::NUMBER(15,3), -789.012::NUMBER(15,3)" is executed
    Then All values should be returned as numpy.float64 type
    And Values should match approximately [0.0, 123.456, -789.012] within float64 precision

  @python_e2e
  Scenario: numpy handles high precision integers within int64 range
    Given Snowflake client is logged in with NumPy mode enabled
    When Query with 18-digit integer (within int64 range) is executed
    Then Value should be numpy.int64
    And Value should match exactly

