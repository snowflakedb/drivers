@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: SQL_C_TYPE_TIMESTAMP bound via SQLBindParameter to SQL_TYPE_TIME

  ODBC-only behavior. SQLBindParameter lets the application bind a
  SQL_C_TYPE_TIMESTAMP source to a SQL_TYPE_TIME target. Per ODBC
  Appendix D ("C to SQL: Timestamp"), the date portion is silently
  discarded and the whole-second h/m/s round-trip — but only when the
  discarded fractional-seconds portion is exactly zero; otherwise the
  driver must return SQL_ERROR with SQLSTATE 22008 ("Datetime field
  overflow"). JDBC and Python expose a single timestamp type per
  direction and cannot exercise this matrix.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # ============================================================================
  # SQL_C_TYPE_TIMESTAMP → TIME — happy paths
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME and discard date component
    Given a TIME column
    When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 is bound and inserted
    Then only the time is preserved and the date is silently discarded

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with non-zero fraction bound to SQL_TYPE_TIME
    Given a TIME column
    When the timestamp carries a half-second fractional component
    Then SQLExecute fails with SQLSTATE 22008

  @odbc_e2e
  Scenario: should bind midnight SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME
    Given a TIME column
    When the timestamp's time portion is 00:00:00
    Then the stored value is the zero time

  @odbc_e2e
  Scenario: should bind end-of-day SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME
    Given a TIME column
    When 23:59:59 on any date is bound
    Then the upper-bound time is preserved

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP at epoch date to SQL_TYPE_TIME
    Given a TIME column
    When the timestamp date is the Unix epoch but the time is non-zero
    Then only the time matters and the epoch date is irrelevant

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_TIME
    Given a TIME column
    When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL

  # ============================================================================
  # Invalid-struct-field rejection — SQLSTATE 22007
  #
  # 22007 takes precedence over 22008 — the
  # *_invalid_*_takes_precedence_over_22008 unit tests in
  # odbc/src/conversion/param_binding.rs pin this for the conversion
  # layer; these e2e scenarios pin the same contract end-to-end.
  # ============================================================================

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with hour=24 bound to SQL_TYPE_TIME
    Given a TIME column
    When the timestamp carries hour=24 which is out of the legal range
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with minute=60 bound to SQL_TYPE_TIME
    Given a TIME column
    When the timestamp carries minute=60 which is out of the legal range
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with out-of-range fraction bound to SQL_TYPE_TIME
    Given a TIME column
    When the timestamp carries fraction=3000000000 ns which is out of the legal range
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should prefer SQLSTATE 22007 over 22008 when SQL_C_TYPE_TIMESTAMP has invalid hour and non-zero fraction
    Given a TIME column
    When the timestamp has both an invalid hour and a non-zero fraction
    Then SQLExecute fails with SQLSTATE 22007 not 22008
