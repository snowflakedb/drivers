@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: SQL_C_INTERVAL_DAY / HOUR / MINUTE / SECOND and composites bound via SQLBindParameter to SQL_VARCHAR

  ODBC-only behavior. Snowflake has no native INTERVAL column type, so
  per ODBC Appendix D ("Converting Data from C to SQL Data Types") all
  SQL_C_INTERVAL_* parameters are routed to a VARCHAR target and
  formatted as the ANSI SQL interval literal text. These scenarios
  exercise the full round-trip:
    SQLPrepare → SQLBindParameter → SQLExecute → SELECT → SQLGetData.

  Format reference, per ODBC "Interval Data Type Length" (every
  non-leading datetime field is rendered as exactly two characters and
  the seconds component carries "1 plus the express or implied seconds
  precision" — defaulting to a 6-digit microsecond fraction):
    DAY                : [-]<day>
    HOUR               : [-]<hour>
    MINUTE             : [-]<minute>
    SECOND             : [-]<second>.<fraction(6)>
    DAY_TO_HOUR        : [-]<day> <hour(2)>
    DAY_TO_MINUTE      : [-]<day> <hour(2)>:<minute(2)>
    DAY_TO_SECOND      : [-]<day> <hour(2)>:<minute(2)>:<second(2)>.<fraction(6)>
    HOUR_TO_MINUTE     : [-]<hour>:<minute(2)>
    HOUR_TO_SECOND     : [-]<hour>:<minute(2)>:<second(2)>.<fraction(6)>
    MINUTE_TO_SECOND   : [-]<minute>:<second(2)>.<fraction(6)>

  `fraction` is in microseconds (matches the unit used elsewhere in the
  driver — see `numeric_helpers::compute_interval_fraction`) and is
  always emitted at the canonical 6-digit width with the decimal
  point, even when the value is zero. This matches both the spec
  literal width and the legacy 3.16.0 driver, so applications can
  round-trip a value through either driver and get an identical
  string.

  JDBC and Python do not surface SQL_C_INTERVAL_* parameter types and
  cannot exercise this matrix; the behavior lives entirely in the
  ODBC driver surface.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # ============================================================================
  # Single-field day/time intervals
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_DAY to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_DAY carrying 15 days is bound and inserted
    Then only the day field is rendered

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_HOUR to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_HOUR carrying 8 hours is bound and inserted
    Then only the hour field is rendered without zero-padding for the leading field

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_MINUTE to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_MINUTE carrying 30 minutes is bound and inserted
    Then only the minute field is rendered

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_SECOND with no fraction to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_SECOND carrying 45 seconds is bound and inserted
    Then the fraction is rendered at the canonical 6-digit width per ODBC "Interval Data Type Length"

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_SECOND with microsecond fraction to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_SECOND carrying 45.500000s is bound and inserted
    Then the fraction is rendered at the canonical 6-digit width matching the legacy driver

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_SECOND with one-microsecond fraction to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_SECOND carrying 1.000001s is bound and inserted
    Then leading-zero microseconds are preserved up to 6 digits

  # ============================================================================
  # Composite day/time intervals
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_DAY_TO_HOUR to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_DAY_TO_HOUR carrying 3 days 7 hours is bound and inserted
    Then the "<day> <hour(2)>" form is stored with the hour zero-padded to 2 digits

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_DAY_TO_MINUTE to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_DAY_TO_MINUTE carrying 3 days 7:05 is bound and inserted
    Then both hour and minute sub-fields are zero-padded to 2 digits

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_DAY_TO_SECOND with fraction to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_DAY_TO_SECOND carrying 10 days 12:30:59.5 is bound and inserted
    Then hour, minute and second are zero-padded and the seconds fraction is rendered at 6-digit microsecond width

  @odbc_e2e
  Scenario: should bind negative SQL_C_INTERVAL_DAY_TO_SECOND to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_DAY_TO_SECOND carrying -1 day 02:03:04 is bound and inserted
    Then the leading sign is applied once and the seconds fraction is emitted at the canonical 6-digit width even when zero

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_HOUR_TO_MINUTE to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_HOUR_TO_MINUTE carrying 14:07 is bound and inserted
    Then the minute sub-field is zero-padded to 2 digits

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_HOUR_TO_SECOND with fraction to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_HOUR_TO_SECOND carrying 12:30:59.25 is bound and inserted
    Then minute and second are zero-padded and the fractional tail is rendered at 6-digit microsecond width

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_MINUTE_TO_SECOND with no fraction to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_MINUTE_TO_SECOND carrying 30:07.000000 is bound and inserted
    Then the second sub-field is zero-padded to 2 digits and the fraction is emitted at the canonical 6-digit width

  # ============================================================================
  # NULL indicator
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_DAY_TO_SECOND with NULL indicator to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_DAY_TO_SECOND is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
