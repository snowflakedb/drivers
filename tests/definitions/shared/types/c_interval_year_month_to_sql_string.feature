@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: SQL_C_INTERVAL_YEAR / MONTH / YEAR_TO_MONTH bound via SQLBindParameter to SQL_VARCHAR

  ODBC-only behavior. Snowflake has no native INTERVAL column type, so
  per ODBC Appendix D ("Converting Data from C to SQL Data Types") all
  SQL_C_INTERVAL_* parameters are routed to a VARCHAR target and
  formatted as the ANSI SQL interval literal text. These scenarios
  exercise the full round-trip:
    SQLPrepare → SQLBindParameter → SQLExecute → SELECT → SQLGetData.

  Format reference (ODBC Appendix D, "C to SQL: Interval"):
    YEAR             : [-]<year>
    MONTH            : [-]<month>
    YEAR_TO_MONTH    : [-]<year>-<month(2)>

  Per ODBC "Interval Data Type Length" every non-leading datetime
  sub-field is rendered as exactly two characters; the leading field
  is unpadded. The driver writes the chosen sub-fields based on the C
  type bound on the parameter — the struct's `interval_type` field is
  intentionally ignored (Appendix D requires conformance to the bound
  C type).

  JDBC and Python do not surface SQL_C_INTERVAL_* parameter types and
  cannot exercise this matrix; the behavior lives entirely in the
  ODBC driver surface.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # ============================================================================
  # SQL_C_INTERVAL_YEAR
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_YEAR to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_YEAR carrying 5 years is bound and inserted
    Then the formatted literal is stored

  @odbc_e2e
  Scenario: should bind negative SQL_C_INTERVAL_YEAR to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_YEAR carrying -7 years is bound and inserted
    Then the leading "-" sign is preserved

  # ============================================================================
  # SQL_C_INTERVAL_MONTH
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_MONTH to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_MONTH carrying 11 months is bound and inserted
    Then the formatted literal is stored without zero-padding for the leading field

  # ============================================================================
  # SQL_C_INTERVAL_YEAR_TO_MONTH
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_YEAR_TO_MONTH carrying 5 years 11 months is bound and inserted
    Then the "<year>-<month(2)>" form is stored

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_YEAR_TO_MONTH with single-digit month to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_YEAR_TO_MONTH carrying 4 years 7 months is bound and inserted
    Then the trailing month sub-field is zero-padded to 2 digits per ODBC "Interval Data Type Length"

  @odbc_e2e
  Scenario: should bind negative SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_YEAR_TO_MONTH carrying -2 years 3 months is bound and inserted
    Then the leading sign is applied once before the year and the trailing month is zero-padded to 2 digits

  # ============================================================================
  # NULL indicator
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_INTERVAL_YEAR with NULL indicator to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_INTERVAL_YEAR is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
