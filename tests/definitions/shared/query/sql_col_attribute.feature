@odbc
Feature: ODBC SQLColAttribute function behavior (ODBC 3.x)
  # Tests for SQLColAttribute based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlcolattribute-function
  #
  # SQLColAttribute is the ODBC 3.x replacement for SQLColAttributes.
  # It uses SQL_DESC_* field identifiers and returns string-valued attributes
  # via CharacterAttributePtr and numeric attributes via NumericAttributePtr.

  # =========================================================================
  # Per-Type Attribute Coverage
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for VARCHAR.
    Given A table with a VARCHAR(100) column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for VARCHAR

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for NUMBER.
    Given A table with a NUMBER(10,2) column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for NUMBER

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for BOOLEAN.
    Given A table with a BOOLEAN column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for BOOLEAN

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for DATE.
    Given A table with a DATE column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for DATE

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for TIME.
    Given A table with a TIME(9) column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for TIME

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for TIMESTAMP_NTZ.
    Given A table with a TIMESTAMP_NTZ(9) column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for TIMESTAMP_NTZ

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for TIMESTAMP_LTZ.
    Given A table with a TIMESTAMP_LTZ(9) column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for TIMESTAMP_LTZ

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for TIMESTAMP_TZ.
    Given A table with a TIMESTAMP_TZ(9) column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for TIMESTAMP_TZ

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for BINARY.
    Given A table with a BINARY column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for BINARY

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for BINARY with explicit size.
    Given A table with a BINARY(100) column is queried
    When SQLColAttribute is called for size-dependent descriptor fields
    Then Size-related attributes should reflect the declared BINARY(100) size

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for FLOAT.
    Given A table with a FLOAT column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for FLOAT

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for DECFLOAT.
    Given A table with a DECFLOAT column is queried
    When SQLColAttribute is called for each descriptor field
    Then All metadata attributes should match expected values for DECFLOAT

  # =========================================================================
  # Column Naming
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns column name via SQL_DESC_NAME.
    Given A query with a named column is executed
    When SQLColAttribute is called with SQL_DESC_NAME
    Then The call should succeed and return the column name

  @odbc_e2e
  Scenario: SQLColAttribute returns column label via SQL_DESC_LABEL.
    Given A query with a labeled column is executed
    When SQLColAttribute is called with SQL_DESC_LABEL
    Then The call should succeed and return the column label

  @odbc_e2e
  Scenario: SQLColAttribute returns empty table/schema/catalog names.
    Given A query is executed
    When SQLColAttribute is called with SQL_DESC_TABLE_NAME, SQL_DESC_BASE_TABLE_NAME, SQL_DESC_SCHEMA_NAME, SQL_DESC_CATALOG_NAME
    Then Each should return an empty string

  @odbc_e2e
  Scenario: SQLColAttribute returns base column name via SQL_DESC_BASE_COLUMN_NAME.
    Given A query with a named column is executed
    When SQLColAttribute is called with SQL_DESC_BASE_COLUMN_NAME
    Then The call should succeed and return the column name

  @odbc_e2e
  Scenario: SQLColAttribute returns column count via SQL_DESC_COUNT.
    Given A multi-column query is executed
    When SQLColAttribute is called with SQL_DESC_COUNT
    Then The call should succeed and return 3

  # =========================================================================
  # Nullable Behavior
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_NULLABLE for nullable column.
    Given A table with a nullable column is queried
    When SQLColAttribute is called with SQL_DESC_NULLABLE
    Then The call should succeed and return SQL_NULLABLE

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_NO_NULLS for NOT NULL column.
    Given A table with a NOT NULL column is queried
    When SQLColAttribute is called with SQL_DESC_NULLABLE
    Then The call should succeed and return SQL_NO_NULLS

  # =========================================================================
  # ODBC 2.x Aliases via SQLColAttribute
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns same values for ODBC 2.x aliases as 3.x equivalents.
    Given A query with a named column is executed
    When SQLColAttribute is called with SQL_COLUMN_NAME and SQL_DESC_NAME
    Then Both should return the same values

  # =========================================================================
  # Prepared State Support
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns column name after SQLPrepare without SQLExecute.
    Given A SELECT statement is prepared but not executed
    When SQLColAttribute is called with SQL_DESC_NAME
    Then The call should succeed and return the column name

  @odbc_e2e
  Scenario: SQLColAttribute returns column count after SQLPrepare without SQLExecute.
    Given A multi-column SELECT is prepared but not executed
    When SQLColAttribute is called with SQL_DESC_COUNT
    Then The call should succeed and return the column count

  @odbc_e2e
  Scenario: SQLColAttribute returns type after SQLPrepare without SQLExecute.
    Given A SELECT returning a numeric literal is prepared but not executed
    When SQLColAttribute is called with SQL_DESC_TYPE
    Then The call should succeed and return the SQL type

  # =========================================================================
  # String Truncation
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_SUCCESS_WITH_INFO with 01004 on string truncation.
    Given A query with a named column is executed
    When SQLColAttribute is called with a buffer too small for the column name
    Then The call should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004 and StringLengthPtr should contain the full untruncated length

  # =========================================================================
  # Error Cases
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns 07009 for column number 0.
    Given A query is executed
    When SQLColAttribute is called with column number 0
    Then The call should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLColAttribute returns 07009 for out-of-range column number.
    Given A single-column query is executed
    When SQLColAttribute is called with a column number beyond the result set
    Then The call should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLColAttribute returns HY010 before prepare or execute.
    Given A statement handle exists but no query has been prepared or executed
    When SQLColAttribute is called without any prepare or execute
    Then The call should return SQL_ERROR with SQLSTATE HY010

  @odbc_e2e
  Scenario: SQLColAttribute returns HY091 for unrecognized field identifier.
    Given A query is executed
    When SQLColAttribute is called with an invalid field identifier
    Then The call should return SQL_ERROR with SQLSTATE HY091

  # =========================================================================
  # Cross-Function Consistency
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute and SQLGetDescField(IRD) return consistent values.
    Given A table with multiple column types is queried
    When Both functions are called for each column and numeric descriptor field
    Then Both functions should return the same string

  @odbc_e2e
  Scenario: SQLColAttribute and SQLDescribeCol return consistent values.
    Given A table with multiple column types is queried
    When Both functions are called for each column
    Then SQLDescribeCol.ColumnName == SQLColAttribute(SQL_DESC_NAME)

  # =========================================================================
  # NUMBER Precision/Scale Variations
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns correct precision/scale for NUMBER(38,0).
    Given A table with a NUMBER(38,0) column is queried
    When SQLColAttribute is called for precision and scale
    Then Precision should be 38 and scale 0

  @odbc_e2e
  Scenario: SQLColAttribute returns correct precision/scale for NUMBER(1,0).
    Given A table with a NUMBER(1,0) column is queried
    When SQLColAttribute is called for precision and scale
    Then Precision should be 1 and scale 0

  @odbc_e2e
  Scenario: SQLColAttribute returns correct precision/scale for NUMBER(38,18).
    Given A table with a NUMBER(38,18) column is queried
    When SQLColAttribute is called for precision and scale
    Then Precision should be 38 and scale 18

  # =========================================================================
  # TIME/TIMESTAMP Scale Variations
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns correct display size for TIME(0).
    Given A table with a TIME(0) column is queried
    When SQLColAttribute is called for display size
    Then Display size should reflect no fractional seconds (HH:MM:SS = 8)

  @odbc_e2e
  Scenario: SQLColAttribute returns correct display size for TIMESTAMP_NTZ(3).
    Given A table with a TIMESTAMP_NTZ(3) column is queried
    When SQLColAttribute is called for display size
    Then Display size should reflect millisecond precision (YYYY-MM-DD HH:MM:SS.fff = 23)

  # =========================================================================
  # BINARY/VARCHAR Size Edge Cases
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for BINARY(1).
    Given A table with a BINARY(1) column is queried
    When SQLColAttribute is called for size-dependent fields
    Then Size attributes should reflect 1 byte

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for VARCHAR(1).
    Given A table with a VARCHAR(1) column is queried
    When SQLColAttribute is called for size-dependent fields
    Then Size attributes should reflect 1 character

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for VARCHAR(16777216).
    Given A table with a VARCHAR(16777216) column (Snowflake max) is queried
    When SQLColAttribute is called for size-dependent fields
    Then Size attributes should reflect 16MB characters

  # =========================================================================
  # StringLengthPtr Validation
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns correct StringLengthPtr for string attributes.
    Given A table with multiple column types is queried
    When SQL_DESC_TYPE_NAME is queried for each column
    Then StringLengthPtr should match the string length

  # =========================================================================
  # Multi-Column Index Validation
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns correct attributes for each column in a multi-column result.
    Given A table with diverse types is queried
    When SQLColAttribute is called for SQL_DESC_CONCISE_TYPE on each column
    Then Each column index should return the correct type
