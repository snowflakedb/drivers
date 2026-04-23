@python @numpy
Feature: TIMESTAMP_NTZ type NumPy support (Python-specific)

  # TIMESTAMP_NTZ values are returned as numpy.datetime64 with nanosecond
  # resolution ('ns') when NumPy mode is enabled, instead of the default
  # datetime.datetime.
  #
  # Range note: numpy.datetime64[ns] is backed by int64 nanoseconds from the
  # Unix epoch, so it can only represent values between roughly 1677-09-21 and
  # 2262-04-11. Snowflake TIMESTAMP_NTZ itself spans 0001-01-01..9999-12-31,
  # so extreme-boundary tests stay within the numpy ns range.

  @python_e2e
  Scenario Outline: should cast usual timestamp_ntz values to numpy datetime64 with nanosecond resolution
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '<input>'::TIMESTAMP_NTZ" is executed
    Then Value should be numpy.datetime64 with nanosecond resolution
    And Value should match exactly <expected>

    Examples:
      | input                        | expected                       |
      | 2024-01-15 10:30:00          | 2024-01-15T10:30:00            |
      | 1970-01-01 00:00:00          | 1970-01-01T00:00:00            |
      | 2024-01-15 10:30:00.123456   | 2024-01-15T10:30:00.123456     |

  @python_e2e
  Scenario: should preserve nanosecond precision for timestamp_ntz with numpy
    # Unlike Python datetime (capped at microseconds), numpy.datetime64[ns]
    # retains all 9 fractional-second digits without truncation. This is the
    # key value-add of NumPy mode over default mode for TIMESTAMP_NTZ.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ" is executed
    Then Value should be numpy.datetime64 with nanosecond resolution
    And Value should match exactly 2024-01-15T10:30:00.123456789

  @python_e2e
  Scenario: should cast boundary timestamp_ntz values within numpy nanosecond range
    # numpy.datetime64[ns] range: ~1677-09-21 .. ~2262-04-11. Exercise values
    # near both ends that still fit in int64 nanoseconds.
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '1678-01-01 00:00:00'::TIMESTAMP_NTZ, '2261-12-31 23:59:59.999999999'::TIMESTAMP_NTZ" is executed
    Then All values should be returned as numpy.datetime64 type with nanosecond resolution
    And Values should match exactly [1678-01-01T00:00:00, 2261-12-31T23:59:59.999999999]

  @python_e2e
  Scenario: should handle NULL values for timestamp_ntz with numpy
    Given Snowflake client is logged in with NumPy mode enabled
    When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
    Then Non-null values should be returned as numpy.datetime64 type with nanosecond resolution
    And Result should contain [2024-01-15T10:30:00, NULL]
