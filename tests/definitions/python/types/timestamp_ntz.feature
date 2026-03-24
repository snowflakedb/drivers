@python
Feature: TIMESTAMP_NTZ Python-specific behaviour

  # Python datetime is capped at microsecond precision (6 decimal places).
  # Sub-microsecond digits received from Snowflake are silently truncated — not rounded.
  # This is a structural Python limitation, not a driver bug.

  @python_e2e
  Scenario Outline: should truncate nanosecond precision to microseconds for timestamp_ntz
    # Truncation, not rounding: digits 7–9 are discarded regardless of their value.
    # The .999999999 case is the critical proof: rounding would increment the second,
    # truncation does not.
    Given Snowflake client is logged in
    When Query "SELECT '<input>'::TIMESTAMP_NTZ" is executed
    Then Result should contain [<expected>]
    And Values should not have timezone info

    Examples:
      | input                          | expected                    |
      | 2024-01-15 10:30:00.123456789  | 2024-01-15 10:30:00.123456  |
      | 2024-01-15 10:30:00.999999999  | 2024-01-15 10:30:00.999999  |
