@odbc
Feature: ODBC string to interval type conversions
  # Tests converting Snowflake VARCHAR/STRING type to interval ODBC C types:
  # SQL_C_INTERVAL_YEAR, SQL_C_INTERVAL_MONTH, SQL_C_INTERVAL_DAY,
  # SQL_C_INTERVAL_HOUR, SQL_C_INTERVAL_MINUTE, SQL_C_INTERVAL_SECOND,
  # SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_C_INTERVAL_DAY_TO_HOUR,
  # SQL_C_INTERVAL_DAY_TO_MINUTE, SQL_C_INTERVAL_DAY_TO_SECOND,
  # SQL_C_INTERVAL_HOUR_TO_MINUTE, SQL_C_INTERVAL_HOUR_TO_SECOND,
  # SQL_C_INTERVAL_MINUTE_TO_SECOND

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Single-component interval types
  # ============================================================================

  @odbc_e2e
  Scenario Outline: should convert string literals to single-component <c_type>
    Given Snowflake client is logged in
    When Query selecting string literals representing interval values is executed
    Then <c_type> conversions should work

    Examples:
      | c_type                 |
      | SQL_C_INTERVAL_YEAR    |
      | SQL_C_INTERVAL_MONTH   |
      | SQL_C_INTERVAL_DAY     |
      | SQL_C_INTERVAL_HOUR    |
      | SQL_C_INTERVAL_MINUTE  |
      | SQL_C_INTERVAL_SECOND  |

  @odbc_e2e
  Scenario Outline: should convert negative <c_type> string literals
    Given Snowflake client is logged in
    When Query selecting negative interval values is executed
    Then negative <c_type> should be correctly parsed

    Examples:
      | c_type                 |
      | SQL_C_INTERVAL_YEAR    |
      | SQL_C_INTERVAL_MONTH   |
      | SQL_C_INTERVAL_DAY     |

  # ============================================================================
  # SUCCESSFUL CONVERSIONS - Multi-component interval types
  # ============================================================================

  @odbc_e2e
  Scenario: should convert string literals to year-month interval type
    Given Snowflake client is logged in
    When Query selecting year-month interval string is executed
    Then SQL_C_INTERVAL_YEAR_TO_MONTH conversions should work
    And negative year-month should be correctly parsed
    And zero years with months should work

  @odbc_e2e
  Scenario Outline: should convert string literals to compound <c_type>
    Given Snowflake client is logged in
    When Query selecting day-time interval strings is executed
    Then <c_type> conversions should work

    Examples:
      | c_type                           |
      | SQL_C_INTERVAL_DAY_TO_HOUR       |
      | SQL_C_INTERVAL_DAY_TO_MINUTE     |
      | SQL_C_INTERVAL_DAY_TO_SECOND     |
      | SQL_C_INTERVAL_HOUR_TO_MINUTE    |
      | SQL_C_INTERVAL_HOUR_TO_SECOND    |
      | SQL_C_INTERVAL_MINUTE_TO_SECOND  |

  # ============================================================================
  # TRUNCATION WITH INFO - Trailing field truncation (SQLSTATE 01S07)
  # ============================================================================

  @odbc_e2e
  Scenario: should truncate trailing fields when converting interval strings
    Given Snowflake client is logged in
    When Query selecting interval strings with more precision than target type is executed
    Then year-month to year should truncate month field
    And day-second to day should truncate time fields
    And hour-second to hour should truncate minute and second
    And minute-second to minute will lose precision since driver treats it as hour-minute

  @odbc_e2e
  Scenario: should truncate trailing fields in day-time intervals
    Given Snowflake client is logged in
    When Query selecting day-time interval strings with more precision is executed
    Then day-second to day-hour should truncate minute and second
    And day-second to day-minute should truncate second
    And hour-second to hour-minute should truncate second

  # ============================================================================
  # LEADING FIELD PRECISION LOSS - (SQLSTATE 22015)
  # ============================================================================

  @odbc_e2e
  Scenario: should fail when leading field precision is lost for year intervals
    Given Snowflake client is logged in
    # Default leading precision is typically 2 digits for intervals
    When Query selecting interval values with leading field exceeding precision is executed
    Then values exceeding leading field precision should fail with 22015

  @odbc_e2e
  Scenario: should fail when leading field precision is lost for month intervals
    Given Snowflake client is logged in
    When Query selecting interval values with leading field exceeding precision is executed
    Then values exceeding leading field precision should fail with 22015

  @odbc_e2e
  Scenario: should fail when leading field precision is lost for day intervals
    Given Snowflake client is logged in
    When Query selecting interval values with leading field exceeding precision is executed
    Then values exceeding leading field precision should fail with 22015

  @odbc_e2e
  Scenario: should fail when leading field precision is lost for hour intervals
    Given Snowflake client is logged in
    When Query selecting interval values with leading field exceeding precision is executed
    Then values exceeding leading field precision should fail with 22015

  @odbc_e2e
  Scenario: should fail when leading field precision is lost for compound intervals
    Given Snowflake client is logged in
    When Query selecting compound interval values with leading field exceeding precision is executed
    Then values exceeding leading field precision should fail with 22015

  # ============================================================================
  # INVALID INTERVAL VALUES - (SQLSTATE 22018)
  # ============================================================================

  @odbc_e2e
  Scenario: should fail converting invalid interval string formats
    Given Snowflake client is logged in
    When Query selecting invalid interval strings is executed
    Then invalid interval strings should fail with SQLSTATE 22018

  @odbc_e2e
  Scenario: should fail converting malformed interval strings for year-month type
    Given Snowflake client is logged in
    When Query selecting malformed year-month interval strings is executed
    Then malformed year-month strings should fail with SQLSTATE 22018

  @odbc_e2e
  Scenario: should fail converting malformed interval strings for day-time types
    Given Snowflake client is logged in
    When Query selecting malformed day-time interval strings is executed
    Then malformed day-time strings should fail with SQLSTATE 22018

  @odbc_e2e
  Scenario: should fail converting out-of-range component values
    Given Snowflake client is logged in
    # Month > 11 in year-month, hour > 23, minute > 59, second > 59
    When Query selecting interval strings with invalid component ranges is executed
    Then out-of-range month values should fail with SQLSTATE 22018
    And out-of-range time components should overflow to next field

  # ============================================================================
  # EDGE CASES - Whitespace and special formatting
  # ============================================================================

  @odbc_e2e
  Scenario: should handle whitespace in interval strings
    Given Snowflake client is logged in
    When Query selecting interval strings with leading/trailing whitespace is executed
    Then whitespace should be trimmed and values parsed correctly

  @odbc_e2e
  Scenario: should handle zero values in interval strings
    Given Snowflake client is logged in
    When Query selecting zero interval values is executed
    Then zero values should be correctly parsed

  # ============================================================================
  # NULL VALUE HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle NULL string when converting to interval types
    Given Snowflake client is logged in
    When Query selecting NULL is executed
    Then NULL should return SQL_NULL_DATA indicator

  # ============================================================================
  # CONVERSION WITH SQLBindCol
  # ============================================================================

  @odbc_e2e
  Scenario: should convert strings to interval types using SQLBindCol
    # Test successful SQL_C_INTERVAL_YEAR binding
    # Test failed binding for invalid interval string
    Given Snowflake client is logged in
    When Query selecting interval value is executed with SQLBindCol for SQL_C_INTERVAL_YEAR
    Then the bound interval value should match the string representation
    And invalid interval string should fail binding with SQLSTATE 22018

  # ============================================================================
  # FRACTIONAL SECONDS HANDLING
  # ============================================================================

  @odbc_e2e
  Scenario: should handle fractional seconds in interval strings
    Given Snowflake client is logged in
    When Query selecting interval strings with fractional seconds is executed
    Then fractional seconds should be parsed correctly
