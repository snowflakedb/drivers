@odbc
Feature: ODBC string to temporal type conversions
  # Tests converting Snowflake VARCHAR/STRING type to temporal ODBC C types:
  # SQL_C_TYPE_DATE, SQL_C_TYPE_TIME, SQL_C_TYPE_TIMESTAMP

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - String to Date/Time Types
  # ============================================================================

  @odbc_e2e
  Scenario Outline: should convert string literals to <c_type>
    Given Snowflake client is logged in
    When Query selecting string literals representing dates and times is executed
    Then <c_type> conversions should work

    Examples:
      | c_type               |
      | SQL_C_TYPE_DATE      |
      | SQL_C_TYPE_TIME      |
      | SQL_C_TYPE_TIMESTAMP |

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Date/Time strings to TIMESTAMP
  # ============================================================================

  @odbc_e2e
  Scenario: should convert date-only and time-only strings to SQL_C_TYPE_TIMESTAMP
    # Date-only strings get midnight time, time-only strings get today's date
    Given Snowflake client is logged in
    When Query selecting date-only string is executed
    And Data is retrieved as SQL_C_TYPE_TIMESTAMP
    Then the date components should be correctly parsed
    And the time components should default to midnight
    And the date components should default to today's date
    And the time components should be correctly parsed

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Timestamp strings to DATE or TIME
  # ============================================================================

  @odbc_e2e
  Scenario: should convert timestamp string to SQL_C_TYPE_DATE
    # Extracting date component from full timestamp string
    Given Snowflake client is logged in
    When Query selecting timestamp strings is executed
    Then SQL_C_TYPE_DATE should extract the date component (time is truncated)

  @odbc_e2e
  Scenario: should convert timestamp string to SQL_C_TYPE_TIME
    # Extracting time component from full timestamp string
    Given Snowflake client is logged in
    When Query selecting timestamp strings is executed
    Then SQL_C_TYPE_TIME should extract the time component (date is truncated)

  # ============================================================================
  # FAILING CONVERSIONS - Invalid date/time format strings
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting invalid date/time strings
    # SQLSTATE 22018 indicates invalid character value for cast
    Given Snowflake client is logged in
    When Query selecting invalid date/time strings is executed
    Then invalid date/time strings should fail

  # ============================================================================
  # FAILING CONVERSIONS - Impossible date/time values (correct syntax, invalid values)
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting impossible date values
    # Date strings with correct YYYY-MM-DD syntax but semantically impossible values
    Given Snowflake client is logged in
    When Query selecting date strings with correct syntax but impossible values is executed
    Then impossible date values should fail with SQLSTATE 22018

  @odbc_e2e
  Scenario: should fail converting impossible time values
    # Time strings with correct HH:MM:SS syntax but semantically impossible values
    Given Snowflake client is logged in
    When Query selecting time strings with correct syntax but impossible values is executed
    Then hour 25 should fail
    And hour 24 should fail
    And minute 60 should fail
    And second 60 might behave differently in the old driver

  @odbc_e2e
  Scenario: should fail converting impossible timestamp values
    # Timestamp strings with correct syntax but semantically impossible values
    Given Snowflake client is logged in
    When Query selecting timestamp strings with correct syntax but impossible values is executed
    Then impossible timestamp values should fail with SQLSTATE 22018

  # ============================================================================
  # FAILING CONVERSIONS - Alternative date serialization formats
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting alternative date formats to SQL_C_TYPE_DATE
    # Tests various non-standard date formats that should fail conversion
    # Only YYYY-MM-DD format is accepted
    Given Snowflake client is logged in
    When Query selecting multiple date strings in alternative formats is executed
    Then all alternative date formats should fail with SQLSTATE 22018

  # ============================================================================
  # FAILING CONVERSIONS - Alternative time serialization formats
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting alternative time formats to SQL_C_TYPE_TIME
    # Tests various non-standard time formats that should fail conversion
    # Only HH:MM:SS format is accepted
    Given Snowflake client is logged in
    When Query selecting multiple time strings in alternative formats is executed
    Then all alternative time formats should fail with SQLSTATE 22018

  # ============================================================================
  # FAILING CONVERSIONS - Alternative timestamp serialization formats
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting alternative timestamp formats to SQL_C_TYPE_TIMESTAMP
    # Tests various non-standard timestamp formats that should fail conversion
    # Only YYYY-MM-DD HH:MM:SS format is accepted
    Given Snowflake client is logged in
    When Query selecting multiple timestamp strings in alternative formats is executed
    Then all alternative timestamp formats should fail with SQLSTATE 22018
