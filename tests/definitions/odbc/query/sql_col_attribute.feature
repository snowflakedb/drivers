@odbc
Feature: ODBC SQLColAttribute function behavior (ODBC 3.x)
  # Tests for SQLColAttribute based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlcolattribute-function
  #
  # SQLColAttribute is the ODBC 3.x replacement for SQLColAttributes.
  # It uses SQL_DESC_* field identifiers and returns string-valued attributes
  # via CharacterAttributePtr and numeric attributes via NumericAttributePtr.

  # =========================================================================
  # String-Valued Field Identifiers
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
  Scenario: SQLColAttribute returns type name for VARCHAR via SQL_DESC_TYPE_NAME.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_TYPE_NAME
    Then The call should succeed and return a non-empty type name

  @odbc_e2e
  Scenario: SQLColAttribute returns type name for NUMBER via SQL_DESC_TYPE_NAME.
    Given A table with a NUMBER column is queried
    When SQLColAttribute is called with SQL_DESC_TYPE_NAME
    Then The call should succeed and return a non-empty type name

  @odbc_e2e
  Scenario: SQLColAttribute returns base column name via SQL_DESC_BASE_COLUMN_NAME.
    Given A table column is queried directly
    When SQLColAttribute is called with SQL_DESC_BASE_COLUMN_NAME
    Then The call should succeed

  @odbc_e2e
  Scenario: SQLColAttribute returns table name via SQL_DESC_TABLE_NAME.
    Given A table column is queried directly
    When SQLColAttribute is called with SQL_DESC_TABLE_NAME
    Then The call should succeed

  @odbc_e2e
  Scenario: SQLColAttribute returns base table name via SQL_DESC_BASE_TABLE_NAME.
    Given A table column is queried directly
    When SQLColAttribute is called with SQL_DESC_BASE_TABLE_NAME
    Then The call should succeed

  @odbc_e2e
  Scenario: SQLColAttribute returns catalog name via SQL_DESC_CATALOG_NAME.
    Given A query is executed
    When SQLColAttribute is called with SQL_DESC_CATALOG_NAME
    Then The call should succeed

  @odbc_e2e
  Scenario: SQLColAttribute returns schema name via SQL_DESC_SCHEMA_NAME.
    Given A query is executed
    When SQLColAttribute is called with SQL_DESC_SCHEMA_NAME
    Then The call should succeed

  @odbc_e2e
  Scenario: SQLColAttribute returns literal prefix for VARCHAR via SQL_DESC_LITERAL_PREFIX.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_LITERAL_PREFIX
    Then The call should succeed

  @odbc_e2e
  Scenario: SQLColAttribute returns literal suffix for VARCHAR via SQL_DESC_LITERAL_SUFFIX.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_LITERAL_SUFFIX
    Then The call should succeed

  @odbc_e2e
  Scenario: SQLColAttribute returns local type name via SQL_DESC_LOCAL_TYPE_NAME.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_LOCAL_TYPE_NAME
    Then The call should succeed

  # =========================================================================
  # Numeric Field Identifiers
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_DECIMAL type for numeric literal.
    Given A query returning a numeric literal is executed
    When SQLColAttribute is called with SQL_DESC_TYPE
    Then The call should succeed and return SQL_DECIMAL

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_VARCHAR concise type for VARCHAR.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_CONCISE_TYPE
    Then The call should succeed and return SQL_VARCHAR

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

  @odbc_e2e
  Scenario: SQLColAttribute returns precision for NUMBER(10,2).
    Given A table with a NUMBER(10,2) column is queried
    When SQLColAttribute is called with SQL_DESC_PRECISION
    Then The call should succeed and return 10

  @odbc_e2e
  Scenario: SQLColAttribute returns scale for NUMBER(10,4).
    Given A table with a NUMBER(10,4) column is queried
    When SQLColAttribute is called with SQL_DESC_SCALE
    Then The call should succeed and return 4

  @odbc_e2e
  Scenario: SQLColAttribute returns length for VARCHAR(200).
    Given A table with a VARCHAR(200) column is queried
    When SQLColAttribute is called with SQL_DESC_LENGTH
    Then The call should succeed and return 200

  @odbc_e2e
  Scenario: SQLColAttribute returns positive octet length for VARCHAR.
    Given A table with a VARCHAR(100) column is queried
    When SQLColAttribute is called with SQL_DESC_OCTET_LENGTH
    Then The call should succeed and return a positive value

  @odbc_e2e
  Scenario: SQLColAttribute returns reasonable display size for VARCHAR.
    Given A table with a VARCHAR(100) column is queried
    When SQLColAttribute is called with SQL_DESC_DISPLAY_SIZE
    Then The call should succeed and return a value >= 100

  @odbc_e2e
  Scenario: SQLColAttribute returns display size for NUMBER.
    Given A table with a NUMBER(10,2) column is queried
    When SQLColAttribute is called with SQL_DESC_DISPLAY_SIZE
    Then The call should succeed and return a positive value

  @odbc_e2e
  Scenario: SQLColAttribute returns num prec radix of 10 for NUMBER.
    Given A table with a NUMBER(10,2) column is queried
    When SQLColAttribute is called with SQL_DESC_NUM_PREC_RADIX
    Then The call should succeed and return 10

  @odbc_e2e
  Scenario: SQLColAttribute returns num prec radix of 0 for VARCHAR.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_NUM_PREC_RADIX
    Then The call should succeed and return 0

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_FALSE for signed numeric column.
    Given A table with a NUMBER column is queried
    When SQLColAttribute is called with SQL_DESC_UNSIGNED
    Then The call should succeed and return SQL_FALSE

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_TRUE for non-numeric column.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_UNSIGNED
    Then The call should succeed and return SQL_TRUE

  @odbc_e2e
  Scenario: SQLColAttribute returns searchable value for column.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_SEARCHABLE
    Then The call should succeed and return a searchable classification

  @odbc_e2e
  Scenario: SQLColAttribute returns updatability for column.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_UPDATABLE
    Then The call should succeed and return a valid updatability value

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_FALSE for auto-unique-value on regular column.
    Given A table with an INT column is queried
    When SQLColAttribute is called with SQL_DESC_AUTO_UNIQUE_VALUE
    Then The call should succeed and return SQL_FALSE

  @odbc_e2e
  Scenario: SQLColAttribute returns case sensitivity for VARCHAR.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_CASE_SENSITIVE
    Then The call should succeed and return SQL_TRUE

  @odbc_e2e
  Scenario: SQLColAttribute returns SQL_FALSE for fixed-prec-scale on VARCHAR.
    Given A table with a VARCHAR column is queried
    When SQLColAttribute is called with SQL_DESC_FIXED_PREC_SCALE
    Then The call should succeed and return SQL_FALSE

  @odbc_e2e
  Scenario: SQLColAttribute returns column count via SQL_DESC_COUNT.
    Given A multi-column query is executed
    When SQLColAttribute is called with SQL_DESC_COUNT
    Then The call should succeed and return 3

  # =========================================================================
  # ODBC 2.x Aliases via SQLColAttribute
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns same values for ODBC 2.x aliases as 3.x equivalents.
    Given A query with a named column is executed
    When SQLColAttribute is called with SQL_COLUMN_NAME and SQL_DESC_NAME
    Then Both should return the same column name
    When SQLColAttribute is called with SQL_COLUMN_NULLABLE and SQL_DESC_NULLABLE
    Then Both should return the same nullable value

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
    Then The call should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And StringLengthPtr should contain the full untruncated length

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
  # Multi-Type Coverage
  # =========================================================================

  @odbc_e2e
  Scenario: SQLColAttribute returns correct metadata for multiple data types.
    Given A table with VARCHAR, NUMBER, and BOOLEAN columns is queried
    When SQLColAttribute is called for column 1 (VARCHAR) name
    Then The column name should be STR_COL
    When SQLColAttribute is called for column 1 (VARCHAR) concise type
    Then The type should be SQL_VARCHAR
    When SQLColAttribute is called for column 1 (VARCHAR) length
    Then The length should be 50
    When SQLColAttribute is called for column 1 (VARCHAR) nullable
    Then The column should be nullable
    When SQLColAttribute is called for column 2 (NUMBER) name
    Then The column name should be NUM_COL
    When SQLColAttribute is called for column 2 (NUMBER) concise type
    Then The type should be SQL_DECIMAL
    When SQLColAttribute is called for column 2 (NUMBER) precision
    Then The precision should be 8
    When SQLColAttribute is called for column 2 (NUMBER) scale
    Then The scale should be 2
    When SQLColAttribute is called for column 2 (NUMBER) unsigned
    Then The column should be signed
    When SQLColAttribute is called for column 3 (BOOLEAN) name
    Then The column name should be BOOL_COL
    When SQLColAttribute is called for column 3 (BOOLEAN) concise type
    Then The type should be SQL_BIT
    When SQLColAttribute is called for column count
    Then The count should be 3
