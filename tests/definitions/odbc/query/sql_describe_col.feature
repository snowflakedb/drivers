@odbc
Feature: ODBC SQLDescribeCol function behavior
  # Tests for SQLDescribeCol based on ODBC specification:
  # https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqldescribecol-function

  @odbc_e2e
  Scenario: SQLDescribeCol returns correct column name.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for column 1
    Then the column name should be MY_COLUMN

  @odbc_e2e
  Scenario: SQLDescribeCol returns empty string for expression column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the expression column
    Then the column name should be empty or driver-defined (Snowflake returns a name)

  @odbc_e2e
  Scenario: SQLDescribeCol returns 01004 when column name is truncated.
    Given Snowflake client is logged in
    When SQLDescribeCol is called with a buffer too small for the column name
    Then SQLDescribeCol should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
    And NameLengthPtr should still contain the full untruncated length

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_VARCHAR for VARCHAR column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the VARCHAR column
    Then the data type should be SQL_VARCHAR

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_DECIMAL for NUMBER column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the NUMBER column
    Then the data type should be SQL_DECIMAL

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_BIT for BOOLEAN column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the BOOLEAN column
    Then the data type should be SQL_BIT

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_DOUBLE for FLOAT column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the FLOAT column
    Then the data type should be SQL_DOUBLE

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_TYPE_DATE for DATE column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the DATE column
    Then the data type should be SQL_TYPE_DATE

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_TYPE_TIMESTAMP for TIMESTAMP_NTZ column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the TIMESTAMP_NTZ column
    Then the data type should be SQL_TYPE_TIMESTAMP

  @odbc_e2e
  Scenario: SQLDescribeCol returns correct column size for VARCHAR.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the VARCHAR(200) column
    Then column size should be 200

  @odbc_e2e
  Scenario: SQLDescribeCol returns precision as column size for NUMBER.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the NUMBER(12,3) column
    Then column size should be 12 (precision)

  @odbc_e2e
  Scenario: SQLDescribeCol returns scale as decimal digits for NUMBER.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the NUMBER(10,4) column
    Then decimal digits should be 4 (scale)

  @odbc_e2e
  Scenario: SQLDescribeCol returns 0 decimal digits for non-numeric types.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for the VARCHAR column
    Then decimal digits should be 0

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_NULLABLE for nullable column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for a nullable column
    Then nullable should be SQL_NULLABLE

  @odbc_e2e
  Scenario: SQLDescribeCol returns SQL_NO_NULLS for NOT NULL column.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for a NOT NULL column
    Then nullable should be SQL_NO_NULLS

  @odbc_e2e
  Scenario: SQLDescribeCol returns 07009 for column number 0 without bookmarks.
    Given Snowflake client is logged in
    When SQLDescribeCol is called with ColumnNumber = 0
    Then SQLDescribeCol should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLDescribeCol returns 07009 for out-of-range column number.
    Given Snowflake client is logged in
    When SQLDescribeCol is called with ColumnNumber > number of columns
    Then SQLDescribeCol should return SQL_ERROR with SQLSTATE 07009

  @odbc_e2e
  Scenario: SQLDescribeCol returns HY090 when BufferLength is less than 0.
    Given Snowflake client is logged in
    When SQLDescribeCol is called with BufferLength < 0
    Then SQLDescribeCol should return SQL_ERROR with SQLSTATE HY090

  @odbc_e2e
  Scenario: SQLDescribeCol returns HY010 when called before prepare or execute.
    Given Snowflake client is logged in
    When SQLDescribeCol is called without any prepare or execute
    Then SQLDescribeCol should return SQL_ERROR with SQLSTATE HY010

  @odbc_e2e
  Scenario: SQLDescribeCol with NULL ColumnName still returns NameLengthPtr.
    Given Snowflake client is logged in
    When SQLDescribeCol is called with NULL ColumnName
    Then NameLengthPtr should contain the full column name length

  @odbc_e2e
  Scenario: SQLDescribeCol succeeds when all output pointers are NULL.
    Given Snowflake client is logged in
    When SQLDescribeCol is called with all output pointers NULL
    Then SQLDescribeCol should succeed

  @odbc_e2e
  Scenario: SQLDescribeCol returns metadata after SQLPrepare.
    Given Snowflake client is logged in
    When a SELECT statement is prepared but not executed
    Then SQLDescribeCol should return metadata for the prepared statement

  @odbc_e2e
  Scenario: SQLDescribeCol returns correct metadata for each column in a multi-column result.
    Given Snowflake client is logged in
    When SQLDescribeCol is called for column 1 (VARCHAR)
    Then column 1 should be VARCHAR with size 50
    When SQLDescribeCol is called for column 2 (NUMBER)
    Then column 2 should be DECIMAL with precision 8 and scale 2
    When SQLDescribeCol is called for column 3 (BOOLEAN)
    Then column 3 should be BIT
