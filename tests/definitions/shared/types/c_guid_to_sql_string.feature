@odbc @core_not_needed @jdbc_not_needed @python_not_needed
Feature: SQL_C_GUID bound via SQLBindParameter to SQL_VARCHAR

  ODBC-only behavior. Snowflake has no native GUID column type, so per
  ODBC Appendix D ("Converting Data from C to SQL Data Types"),
  SQL_C_GUID → SQL_VARCHAR / SQL_CHAR / SQL_WCHAR is the canonical text
  route. The driver formats the 16-byte SQLGUID as the standard
  8-4-4-4-12 hex literal in upper case (Data1 and Data2/Data3 are
  little-endian integer fields, Data4 is a fixed byte sequence) — see
  `varchar.rs::WriteODBCType for SnowflakeVarchar` for the format
  string. JDBC and Python do not surface a SQL_C_GUID parameter type
  and cannot exercise this matrix; the behavior lives entirely in the
  ODBC driver surface.
  Reference:
    https://learn.microsoft.com/sql/odbc/reference/appendixes/converting-data-from-c-to-sql-data-types

  # ============================================================================
  # SQL_C_GUID → VARCHAR — round-trip happy paths
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_GUID to SQL_VARCHAR
    Given a VARCHAR column wide enough for the canonical 36-char form
    When a canonical GUID is bound and inserted
    Then the formatted literal is the canonical 8-4-4-4-12 upper-case hex form

  @odbc_e2e
  Scenario: should bind nil SQL_C_GUID to SQL_VARCHAR
    Given a VARCHAR column
    When the all-zero "nil" GUID is bound and inserted
    Then every section is rendered with full-width zero padding rather than collapsed

  @odbc_e2e
  Scenario: should bind max SQL_C_GUID to SQL_VARCHAR
    Given a VARCHAR column
    When an all-`F` GUID is bound and inserted
    Then every section is rendered as the maximum hex value

  # ============================================================================
  # NULL indicator
  # ============================================================================

  @odbc_e2e
  Scenario: should bind SQL_C_GUID with NULL indicator to SQL_VARCHAR
    Given a VARCHAR column
    When SQL_C_GUID is bound with SQL_NULL_DATA and inserted
    Then the stored value should be NULL
