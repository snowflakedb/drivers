#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"

// =============================================================================
// Tests for SQLNumResultCols based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function
// =============================================================================

// =============================================================================
// Happy Path
// =============================================================================

TEST_CASE("SQLNumResultCols returns 1 for SELECT with single column.", "[query]") {
  // Doc: "SQLNumResultCols returns the number of columns in a result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#summary

  // Given Snowflake client is logged in
  Connection conn;

  // When a SELECT query with one column is executed
  auto stmt = conn.execute("SELECT 42 AS value");

  // Then SQLNumResultCols should return 1
  SQLSMALLINT num_cols = 0;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 1);
}

TEST_CASE("SQLNumResultCols returns correct count for SELECT with many columns.", "[query]") {
  // Doc: "SQLNumResultCols returns the number of columns in a result set."
  // Doc: "This count does not include a bound bookmark column."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#arguments

  // Given Snowflake client is logged in
  Connection conn;

  // When a SELECT query with 5 columns is executed
  auto stmt = conn.execute("SELECT 1 AS a, 2 AS b, 3 AS c, 4 AS d, 5 AS e");

  // Then SQLNumResultCols should return 5
  SQLSMALLINT num_cols = 0;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 5);
}

TEST_CASE("SQLNumResultCols returns correct count for SELECT * from table.", "[query]") {
  // Doc: "SQLNumResultCols returns the number of columns in a result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And a table with 3 columns exists
  conn.execute("CREATE TABLE num_cols_test (id INT, name VARCHAR(100), active BOOLEAN)");

  // When SELECT * is executed on the table
  auto stmt = conn.execute("SELECT * FROM num_cols_test");

  // Then SQLNumResultCols should return 3
  SQLSMALLINT num_cols = 0;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 3);
}

// =============================================================================
// Empty Result Set
// =============================================================================

TEST_CASE("SQLNumResultCols returns correct column count for empty result set.", "[query]") {
  // Doc: "SQLNumResultCols returns the number of columns in a result set."
  // Doc: "If the statement associated with StatementHandle does not return columns,
  //       SQLNumResultCols sets *ColumnCountPtr to 0."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#comments

  // Given Snowflake client is logged in
  Connection conn;

  // When a SELECT query with WHERE 1=0 is executed (empty result set but with columns)
  auto stmt = conn.execute("SELECT 1 AS col1, 2 AS col2 WHERE 1=0");

  // Then SQLNumResultCols should still return the column count (2)
  SQLSMALLINT num_cols = 0;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 2);
}

// =============================================================================
// DDL / No Result Set
// =============================================================================

TEST_CASE("SQLNumResultCols returns 0 after DDL statement.", "[query]") {
  // Doc: "If the statement associated with StatementHandle does not return columns,
  //       SQLNumResultCols sets *ColumnCountPtr to 0."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  auto stmt = conn.createStatement();

  // When a DDL statement is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"CREATE TABLE ddl_numcols_test (id INT)", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SQLNumResultCols should return 0 (DDL produces no result set columns)
  SQLSMALLINT num_cols = -1;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 0);
}

TEST_CASE("SQLNumResultCols returns correct count after calling a stored procedure.", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And a stored procedure exists that returns one column
  conn.execute(
      "CREATE PROCEDURE numcols_proc(p1 VARCHAR) "
      "RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END'");

  // When the stored procedure is called
  auto stmt = conn.execute("CALL numcols_proc('hello')");

  // Then SQLNumResultCols should return 1
  SQLSMALLINT num_cols = 0;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 1);
}

// =============================================================================
// State Errors
// =============================================================================

TEST_CASE("SQLNumResultCols returns HY010 when called on freshly allocated statement.", "[query]") {
  // Doc: "(DM) The function was called prior to calling SQLPrepare or SQLExecDirect
  //       for the StatementHandle."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLNumResultCols is called without any prepare or execute
  SQLSMALLINT num_cols = 0;
  SQLRETURN ret = SQLNumResultCols(stmt.getHandle(), &num_cols);

  // Then it should return SQL_ERROR with SQLSTATE HY010
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY010");
}

// =============================================================================
// After SQLPrepare (before SQLExecute)
// =============================================================================

TEST_CASE("SQLNumResultCols returns column count after SQLPrepare.", "[query]") {
  // Doc: "SQLNumResultCols can be called successfully only when the statement is
  //       in the prepared, executed, or positioned state."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#comments
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT statement is prepared but not executed
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS a, 2 AS b, 3 AS c", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SQLNumResultCols should return the column count
  SQLSMALLINT num_cols = 0;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 3);
}

// =============================================================================
// Re-execution
// =============================================================================

TEST_CASE("SQLNumResultCols updates column count after re-execution with different query.", "[query]") {
  // Doc: "SQLNumResultCols returns the number of columns in a result set."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query with 3 columns is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS a, 2 AS b, 3 AS c", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLNumResultCols is called
  SQLSMALLINT num_cols = 0;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 3);

  // And the cursor is closed before re-executing
  ret = SQLCloseCursor(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // And a different query with 1 column is executed on the same statement
  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS only_col", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then SQLNumResultCols should return the updated column count
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);
  CHECK(num_cols == 1);
}

// =============================================================================
// IRD Consistency
// =============================================================================

TEST_CASE("SQLNumResultCols returns same value as SQL_DESC_COUNT of the IRD.", "[query]") {
  // Doc: "The number of columns returned by SQLNumResultCols is the same value
  //       as the SQL_DESC_COUNT field of the IRD."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query with 4 columns is executed
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS a, 2 AS b, 3 AS c, 4 AS d", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // And SQLNumResultCols is called
  SQLSMALLINT num_cols = 0;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  CHECK_ODBC(ret, stmt);

  // And the IRD SQL_DESC_COUNT is read
  SQLHDESC ird = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_ROW_DESC, &ird, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  SQLSMALLINT ird_count = 0;
  ret = SQLGetDescField(ird, 0, SQL_DESC_COUNT, &ird_count, 0, NULL);
  REQUIRE(ret == SQL_SUCCESS);

  // Then both values should be equal
  CHECK(num_cols == ird_count);
  CHECK(num_cols == 4);
}
