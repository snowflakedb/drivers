@python @numpy
Feature: DATE type NumPy support (Python-specific)

  # DATE values are returned as numpy.datetime64 with day resolution ('D')
  # when NumPy mode is enabled, instead of the default datetime.date.

  @python_e2e
  Scenario: should cast usual date values to numpy datetime64 with day resolution
    # Covers epoch, pre-epoch, and a modern date.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '2024-01-15'::DATE" is executed
    Then All values should be returned as numpy.datetime64 type with day resolution
    And Values should match exactly [1970-01-01, 1969-12-31, 2024-01-15]

  @python_e2e
  Scenario: should cast boundary date values to numpy datetime64 with day resolution
    # Snowflake DATE spec min/max: 0001-01-01 and 9999-12-31.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '0001-01-01'::DATE, '9999-12-31'::DATE" is executed
    Then All values should be returned as numpy.datetime64 type with day resolution
    And Values should match exactly [0001-01-01, 9999-12-31]

  @python_e2e
  Scenario: should handle NULL values for date with numpy
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
    Then Non-null values should be returned as numpy.datetime64 type with day resolution
    And Result should contain [NULL, 2024-01-15, NULL]
