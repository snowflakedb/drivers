@odbc
Feature: ODBC SQLColAttributes function behavior (ODBC 2.x)
  # Tests for SQLColAttributes based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlcolattributes-function
  # https://learn.microsoft.com/en-us/sql/odbc/reference/appendixes/sqlcolattributes-mapping

  @odbc_e2e
  Scenario: SQLColAttributes returns correct column name via SQL_COLUMN_NAME.
    Given A query with a named column is executed
    When SQLColAttributes is called with SQL_COLUMN_NAME
    Then The call should succeed and return the column name

  @odbc_e2e
  Scenario: SQLColAttributes returns SQL_DECIMAL for numeric literal via SQL_COLUMN_TYPE.
    Given A query returning a numeric literal is executed
    When SQLColAttributes is called with SQL_COLUMN_TYPE
    Then The call should succeed and return SQL_DECIMAL

  @odbc_e2e
  Scenario: SQLColAttributes returns SQL_VARCHAR for VARCHAR column via SQL_COLUMN_TYPE.
    Given A table with a VARCHAR column is queried
    When SQLColAttributes is called with SQL_COLUMN_TYPE
    Then The call should succeed and return SQL_VARCHAR

  @odbc_e2e
  Scenario: SQLColAttributes returns transfer octet length for VARCHAR via SQL_COLUMN_LENGTH.
    Given A table with a VARCHAR(100) column is queried
    When SQLColAttributes is called with SQL_COLUMN_LENGTH
    Then The call should succeed and return the transfer octet length

  @odbc_e2e
  Scenario: SQLColAttributes returns transfer octet length for NUMBER via SQL_COLUMN_LENGTH.
    Given A table with a NUMBER(10,2) column is queried
    When SQLColAttributes is called with SQL_COLUMN_LENGTH
    Then The call should succeed and return a positive transfer octet length

  @odbc_e2e
  Scenario: SQLColAttributes returns SQL_NULLABLE for nullable column.
    Given A table with a nullable column is queried
    When SQLColAttributes is called with SQL_COLUMN_NULLABLE
    Then The call should succeed and return SQL_NULLABLE

  @odbc_e2e
  Scenario: SQLColAttributes returns SQL_NO_NULLS for NOT NULL column.
    Given A table with a NOT NULL column is queried
    When SQLColAttributes is called with SQL_COLUMN_NULLABLE
    Then The call should succeed and return SQL_NO_NULLS

  @odbc_e2e
  Scenario: SQLColAttributes returns character length for VARCHAR via SQL_COLUMN_PRECISION.
    Given A table with a VARCHAR(200) column is queried
    When SQLColAttributes is called with SQL_COLUMN_PRECISION
    Then The call should succeed and return 200

  @odbc_e2e
  Scenario: SQLColAttributes returns numeric precision for NUMBER via SQL_COLUMN_PRECISION.
    Given A table with a NUMBER(10,2) column is queried
    When SQLColAttributes is called with SQL_COLUMN_PRECISION
    Then The call should succeed and return 10

  @odbc_e2e
  Scenario: SQLColAttributes returns scale for NUMBER via SQL_COLUMN_SCALE.
    Given A table with a NUMBER(10,4) column is queried
    When SQLColAttributes is called with SQL_COLUMN_SCALE
    Then The call should succeed and return 4

  @odbc_e2e
  Scenario: SQLColAttributes returns 0 scale for VARCHAR via SQL_COLUMN_SCALE.
    Given A table with a VARCHAR column is queried
    When SQLColAttributes is called with SQL_COLUMN_SCALE
    Then The call should succeed and return 0

  @odbc_e2e
  Scenario: SQLColAttributes returns column count via SQL_COLUMN_COUNT.
    Given A multi-column query is executed
    When SQLColAttributes is called with SQL_COLUMN_COUNT
    Then The call should succeed and return 3

  @odbc_e2e
  Scenario: SQLColAttributes returns correct metadata for each column in a multi-column result.
    Given A table with VARCHAR, NUMBER, and BOOLEAN columns is queried
    When SQLColAttributes is called for column 1 (VARCHAR) name
    Then The column name should be STR_COL
    When SQLColAttributes is called for column 1 (VARCHAR) type
    Then The type should be SQL_VARCHAR
    When SQLColAttributes is called for column 1 (VARCHAR) precision
    Then The precision (column size) should be 50
    When SQLColAttributes is called for column 2 (NUMBER) name
    Then The column name should be NUM_COL
    When SQLColAttributes is called for column 2 (NUMBER) type
    Then The type should be SQL_DECIMAL
    When SQLColAttributes is called for column 2 (NUMBER) precision
    Then The precision should be 8
    When SQLColAttributes is called for column 2 (NUMBER) scale
    Then The scale should be 2
    When SQLColAttributes is called for column 3 (BOOLEAN) name
    Then The column name should be BOOL_COL
    When SQLColAttributes is called for column 3 (BOOLEAN) type
    Then The type should be SQL_BIT

  @odbc_e2e
  Scenario: SQLColAttributes returns 07009 for column number 0 without bookmarks.
    Given A query is executed
    When SQLColAttributes is called with column number 0 for a non-count attribute
    Then The call should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLColAttributes returns 07009 for out-of-range column number.
    Given A single-column query is executed
    When SQLColAttributes is called with a column number beyond the result set
    Then The call should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLColAttributes returns HY010 when called before prepare or execute.
    Given A statement handle exists but no query has been prepared or executed
    When SQLColAttributes is called without any prepare or execute
    Then The call should return SQL_ERROR with SQLSTATE HY010
