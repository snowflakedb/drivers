@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: SQL_C_TYPE_DATE bound via SQLBindParameter to SQL_TYPE_TIMESTAMP

  ODBC-only behavior. SQLBindParameter lets the application bind a
  SQL_C_TYPE_DATE source to a SQL_TYPE_TIMESTAMP target. Per ODBC
  Appendix D ("C to SQL: Date"), the date round-trips and the time
  portion of the resulting timestamp is set to 00:00:00.000000000. JDBC
  and Python expose a single date type per direction and cannot
  exercise this matrix; the behavior lives entirely in the ODBC driver
  surface.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # ============================================================================
  # SQL_C_TYPE_DATE → TIMESTAMP_NTZ / LTZ / TZ — happy paths
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE to TIMESTAMP_NTZ at midnight
    Given a TIMESTAMP_NTZ column
    When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_TIMESTAMP and inserted
    Then the stored value has the bound date and a zeroed time component

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE to TIMESTAMP_LTZ at midnight UTC
    Given a TIMESTAMP_LTZ column with a known session timezone
    When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_TIMESTAMP and inserted
    Then the stored value has the bound date and midnight UTC

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE to TIMESTAMP_TZ at midnight UTC
    Given a TIMESTAMP_TZ column with a known session timezone
    When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_TIMESTAMP and inserted
    Then the stored value has the bound date and midnight UTC

  # ============================================================================
  # Edge cases — leap day, epoch, NULL indicator
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE leap day 2024-02-29 to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column with a known session timezone
    When the leap day 2024-02-29 is bound and inserted
    Then the leap day is preserved

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE epoch 1970-01-01 to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column with a known session timezone
    When the Unix epoch date is bound and inserted
    Then the epoch date is preserved at midnight

  @odbc_e2e
  Scenario: should bind SQL_C_TYPE_DATE with NULL indicator to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When SQL_C_TYPE_DATE is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL

  # ============================================================================
  # Invalid-struct-field rejection — SQLSTATE 22007
  #
  # Per ODBC Appendix D, a SQL_C_TYPE_DATE struct whose fields are
  # outside their legal range (year < 1, month not in 1..12, day outside
  # 1..days-in-month) must surface SQL_ERROR with SQLSTATE 22007
  # ("Invalid datetime format") — distinct from the 22008 "Datetime
  # field overflow" diagnostic used for narrowing conversions.
  # ============================================================================

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_DATE with month=13 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the date carries month=13 which is out of the legal range
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_DATE with month=0 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the date carries month=0 which is not a valid month index
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_DATE with day=32 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the date carries day=32 and no month has 32 days
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_DATE with day=0 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the date carries day=0 which is not a valid day of month
    Then SQLExecute fails with SQLSTATE 22007

  @odbc_e2e
  Scenario: should reject SQL_C_TYPE_DATE with non-leap-year Feb 29 bound to SQL_TYPE_TIMESTAMP
    Given a TIMESTAMP_NTZ column
    When the date carries Feb 29 for a non-leap year 2023
    Then SQLExecute fails with SQLSTATE 22007
