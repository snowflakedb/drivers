@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: SQL_C_TYPE_TIMESTAMP bound via SQLBindParameter to SQL_TYPE_DATE

  ODBC-only behavior. SQLBindParameter lets the application bind a
  SQL_C_TYPE_TIMESTAMP source to a SQL_TYPE_DATE target. Per ODBC
  Appendix D ("C to SQL: Timestamp"), the conversion only succeeds
  when the discarded time portion is exactly zero; otherwise the driver
  must return SQL_ERROR with SQLSTATE 22008 ("Datetime field overflow")
  rather than silently truncate or round. JDBC and Python expose a
  single timestamp type per direction and cannot exercise this matrix.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # ============================================================================
  # SQL_C_TYPE_TIMESTAMP → DATE — happy paths (time portion = 00:00:00.0)
  # ============================================================================

  @odbc_e2e
  Scenario: should bind midnight SQL_C_TYPE_TIMESTAMP to SQL_TYPE_DATE without info loss
    Given a DATE column
    When SQL_C_TYPE_TIMESTAMP at exactly midnight is bound to a DATE target
    Then the date round-trips exactly

  @odbc_e2e
  Scenario: should bind leap-day SQL_C_TYPE_TIMESTAMP at midnight to SQL_TYPE_DATE
    Given a DATE column
    When the leap day 2024-02-29 at exactly midnight is bound to DATE
    Then the leap date is preserved

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_DATE
    Given a DATE column
    When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL

  # ============================================================================
  # Datetime field overflow — SQLSTATE 22008
  # ============================================================================

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with non-zero time bound to SQL_TYPE_DATE
    Given a DATE column
    When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 with non-zero h/m/s is bound
    Then SQLExecute fails with SQLSTATE 22008

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with non-zero fraction bound to SQL_TYPE_DATE
    Given a DATE column
    When the timestamp carries a non-zero nanosecond fraction with whole seconds zero
    Then SQLExecute fails with SQLSTATE 22008

  @odbc_e2e
  Scenario: should reject end-of-day SQL_C_TYPE_TIMESTAMP bound to SQL_TYPE_DATE (no rollover)
    Given a DATE column
    When 23:59:59 on 2026-04-13 is bound and the conversion must not silently round up
    Then SQLExecute fails with SQLSTATE 22008

  # ============================================================================
  # Invalid-struct-field rejection — SQLSTATE 22007
  #
  # 22007 takes precedence over 22008 — the
  # *_invalid_*_takes_precedence_over_22008 unit tests in
  # odbc/src/conversion/param_binding.rs pin this for the conversion
  # layer; these e2e scenarios pin the same contract end-to-end.
  # ============================================================================

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with month=13 bound to SQL_TYPE_DATE
    Given a DATE column
    When the timestamp carries month=13 which is out of the legal range
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIMESTAMP with day=32 bound to SQL_TYPE_DATE
    Given a DATE column
    When the timestamp carries day=32 and no month has 32 days
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should prefer SQLSTATE 22007 over 22008 when SQL_C_TYPE_TIMESTAMP has invalid month and non-zero time
    Given a DATE column
    When the timestamp has both an invalid date field and a non-zero time portion
    Then SQLExecute fails with SQLSTATE 22007 not 22008
