@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: SQL_C_TYPE_TIME bound via SQLBindParameter to SQL_TYPE_TIMESTAMP

  ODBC-only behavior. SQLBindParameter lets the application bind a
  SQL_C_TYPE_TIME source to a SQL_TYPE_TIMESTAMP target. Per ODBC
  Appendix D ("C to SQL: Time"), the date fields of the resulting
  timestamp are set to the current local date and the fractional
  seconds portion is zero. JDBC and Python expose a single time type
  per direction and cannot exercise this matrix.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # ============================================================================
  # SQL_C_TYPE_TIME → TIMESTAMP_NTZ / LTZ / TZ — happy paths
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME to TIMESTAMP_NTZ with current local date
    Given a TIMESTAMP_NTZ column
    When SQL_C_TYPE_TIME 14:30:45 is bound to SQL_TYPE_TIMESTAMP and inserted
    Then the time round-trips exactly the fraction is zero and the date falls within the local clock window at bind time

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME to TIMESTAMP_LTZ with current local date
    Given a TIMESTAMP_LTZ column with a known session timezone
    When SQL_C_TYPE_TIME 14:30:45 is bound and inserted
    Then the bind succeeds and the time component round-trips

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME to TIMESTAMP_TZ with current local date
    Given a TIMESTAMP_TZ column with a known session timezone
    When SQL_C_TYPE_TIME 14:30:45 is bound and inserted
    Then the bind succeeds and the time component round-trips

  # ============================================================================
  # NULL indicator
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_TIME with NULL indicator to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When SQL_C_TYPE_TIME is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL

  # ============================================================================
  # Invalid-struct-field rejection — SQLSTATE 22007
  #
  # Per ODBC Appendix D, a SQL_C_TYPE_TIME struct whose fields are
  # outside their legal range (hour not in 0..23, minute or second not
  # in 0..59) must surface SQL_ERROR with SQLSTATE 22007 ("Invalid
  # datetime format").
  # ============================================================================

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIME with hour=24 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the time carries hour=24 which is out of the legal range
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIME with minute=60 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the time carries minute=60 which is out of the legal range
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_TIME with second=60 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the time carries second=60 and Snowflake does not honor leap seconds
    Then SQLExecute fails with SQLSTATE 22007
