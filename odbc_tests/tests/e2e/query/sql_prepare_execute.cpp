#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"

// =============================================================================
// Tests for SQLPrepare / SQLExecute / SQLExecDirect based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecute-function
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecdirect-function
// =============================================================================

// =============================================================================
// SQLPrepare Basic Functionality
// =============================================================================

TEST_CASE("SQLPrepare + SQLExecute retrieves result from simple SELECT.", "[query][prepare]") {
  // Doc: "SQLPrepare prepares an SQL string for execution."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a simple SELECT is prepared and executed
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should contain the expected value
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 42);
}

TEST_CASE("SQLPrepare + SQLExecute retrieves result with multiple columns.", "[query][prepare]") {
  // Doc: "SQLPrepare prepares an SQL string for execution."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT with multiple columns is prepared and executed
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS a, 'hello' AS b, 3.14 AS c", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then all columns should be retrievable
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "hello");
  CHECK(get_data<SQL_C_CHAR>(stmt, 3) == "3.14");
}

TEST_CASE("SQLPrepare + SQLExecute can be executed multiple times with SQLCloseCursor between.", "[query][prepare]") {
  // Doc: "Once a statement is prepared, the application can request that the Driver
  //       Manager or driver include information about the result set. ... The application
  //       can reuse the SQL statement by calling SQLExecute with new parameter values."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 100 AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And executed a first time
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 100);

  // And the cursor is closed
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And executed a second time
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the same result should be returned
  value = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 100);
}

TEST_CASE("Re-prepare replaces previous statement on same handle.", "[query][prepare]") {
  // Doc: "If the statement handle previously contained an SQL statement, the driver
  //       discards the previous statement and compiles the new one."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is prepared and replaced with a different query
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 999 AS replaced_value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should come from the second prepared statement
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 999);
}

TEST_CASE("SQLPrepare with explicit text length (not SQL_NTS).", "[query][prepare]") {
  // Doc: "TextLength [Input] Length of *StatementText in characters."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is prepared with explicit text length
  const char* sql = "SELECT 77 AS value";
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)sql, (SQLINTEGER)strlen(sql));
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should be correct
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 77);
}

TEST_CASE("SQLPrepare with explicit length shorter than string uses partial SQL.", "[query][prepare]") {
  // Doc: "TextLength [Input] Length of *StatementText in characters."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#arguments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is prepared with a length shorter than the full string
  const char* sql = "SELECT 55 AS value";  // 18 chars; passing only 9 gives "SELECT 55" which is still valid
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)sql, 9);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should reflect the truncated query
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 55);
}

TEST_CASE("SQLNumResultCols available after SQLPrepare without execute.", "[query][prepare]") {
  // Doc: "SQLNumResultCols can be called successfully only when the statement is
  //       in the prepared, executed, or positioned state."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlnumresultcols-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT with 3 columns is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS a, 2 AS b, 3 AS c", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLNumResultCols should return the column count without needing execute
  SQLSMALLINT num_cols = 0;
  ret = SQLNumResultCols(stmt.getHandle(), &num_cols);
  REQUIRE_ODBC(ret, stmt);
  CHECK(num_cols == 3);
}

TEST_CASE("SQLDescribeCol available after SQLPrepare without execute.", "[query][prepare]") {
  // Doc: "An application typically calls SQLDescribeCol after a call to SQLPrepare
  //       and before or after the associated call to SQLExecute."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqldescribecol-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 42 AS MY_COL", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLDescribeCol should return metadata for the prepared column
  SQLCHAR col_name[128] = {0};
  SQLSMALLINT name_length = 0;
  SQLSMALLINT data_type = 0;
  SQLULEN col_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeCol(stmt.getHandle(), 1, col_name, sizeof(col_name), &name_length, &data_type, &col_size,
                       &decimal_digits, &nullable);
  REQUIRE_ODBC(ret, stmt);
  CHECK(std::string((char*)col_name) == "MY_COL");
}

// =============================================================================
// SQLPrepareW (wide variant)
// =============================================================================

TEST_CASE("SQLPrepareW + SQLExecute basic flow.", "[query][prepare]") {
  // Doc: "SQLPrepareW is the Unicode version of SQLPrepare."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is prepared using the wide variant
  std::u16string sql = u"SELECT 88 AS wide_value";
  SQLRETURN ret = SQLPrepareW(stmt.getHandle(), (SQLWCHAR*)sql.data(), (SQLINTEGER)sql.size());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should be correct
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 88);
}

TEST_CASE("SQLPrepareW with Unicode content in query.", "[query][prepare]") {
  SKIP_WINDOWS_STRING_ENCODING();
  // Doc: "SQLPrepareW is the Unicode version of SQLPrepare."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT with Unicode string literal is prepared using SQLPrepareW
  std::u16string sql = u"SELECT '日本語テスト' AS unicode_col";
  SQLRETURN ret = SQLPrepareW(stmt.getHandle(), (SQLWCHAR*)sql.data(), (SQLINTEGER)sql.size());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the Unicode content should be correctly returned
  CHECK(get_data<SQL_C_WCHAR>(stmt, 1) == u"日本語テスト");
}

// =============================================================================
// SQLExecute Behavior
// =============================================================================

TEST_CASE("SQLExecute without prior SQLPrepare returns HY010.", "[query][prepare][error]") {
  // Doc: "HY010 - Function sequence error: ... The StatementHandle was not prepared."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecute-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecute is called without a prior SQLPrepare
  SQLRETURN ret = SQLExecute(stmt.getHandle());

  // Then it should return SQL_ERROR with SQLSTATE HY010
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY010");
}

TEST_CASE("SQLExecute with bound parameters via SQLBindParameter.", "[query][prepare]") {
  // Doc: "If the statement contains parameter markers, the application calls
  //       SQLBindParameter to bind each parameter to an application variable."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized query is prepared and parameters are bound
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::VARCHAR AS param_val", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  std::string param = "bound_value";
  SQLLEN param_len = param.size();
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, param.size(), 0,
                         (SQLCHAR*)param.c_str(), param.size(), &param_len);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the bound parameter value should be returned
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "bound_value");
}

TEST_CASE("SQLExecute with different parameter values on re-execution.", "[query][prepare]") {
  // Doc: "The application can reuse the SQL statement by calling SQLExecute
  //       with new parameter values."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized query is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT ?::INTEGER AS val", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER param_value = 10;
  SQLLEN param_len = sizeof(param_value);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &param_value,
                         sizeof(param_value), &param_len);
  REQUIRE_ODBC(ret, stmt);

  // And executed with value 10
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 10);

  // And cursor is closed
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And re-executed with value 20
  param_value = 20;
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the new parameter value should be returned
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 20);
}

TEST_CASE("SQLExecute returns 24000 when cursor is not closed before re-execute of SELECT.",
          "[query][prepare][error]") {
  // Doc: "24000 - Invalid cursor state: A cursor was open on the StatementHandle."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecute-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is prepared and executed
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecute is called again without closing the cursor
  ret = SQLExecute(stmt.getHandle());

  // Then it should return SQL_ERROR with SQLSTATE 24000
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "24000");
}

// =============================================================================
// SQLExecDirect Enhancements
// =============================================================================

TEST_CASE("SQLExecDirectW basic flow.", "[query][prepare]") {
  // Doc: "SQLExecDirect submits an SQL statement for one-time execution."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecdirect-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT is executed via SQLExecDirectW
  std::u16string sql = u"SELECT 123 AS direct_w_val";
  SQLRETURN ret = SQLExecDirectW(stmt.getHandle(), (SQLWCHAR*)sql.data(), (SQLINTEGER)sql.size());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should be correct
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
  REQUIRE_ODBC(ret, stmt);
  CHECK(value == 123);
}

TEST_CASE("SQLExecDirect with bound parameters via SQLBindParameter.", "[query][prepare]") {
  // Doc: "If the statement contains parameter markers, the application uses
  //       SQLBindParameter to bind each parameter to an application variable.
  //       These steps must be done before passing the SQL statement to SQLExecDirect."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecdirect-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameter is bound before calling SQLExecDirect
  std::string param = "direct_bound";
  SQLLEN param_len = param.size();
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, param.size(), 0,
                                   (SQLCHAR*)param.c_str(), param.size(), &param_len);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT ?::VARCHAR AS bound_val", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the bound parameter value should be returned
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "direct_bound");
}

// =============================================================================
// SQLPrepare Error Cases
// =============================================================================

TEST_CASE("SQLPrepare with null statement handle returns SQL_INVALID_HANDLE.", "[query][prepare][error]") {
  // Doc: "SQL_INVALID_HANDLE - The StatementHandle was not a valid statement handle."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#returns

  // When SQLPrepare is called with a null statement handle
  SQLRETURN ret = SQLPrepare(SQL_NULL_HSTMT, (SQLCHAR*)"SELECT 1", SQL_NTS);

  // Then it should return SQL_INVALID_HANDLE
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLPrepare with null SQL text pointer returns HY009.", "[query][prepare][error]") {
  // Doc: "HY009 - Invalid use of null pointer: The argument StatementText was a null pointer."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare is called with a null SQL text pointer
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), nullptr, SQL_NTS);

  // Then it should return SQL_ERROR with SQLSTATE HY009
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY009");
}

TEST_CASE("SQLPrepare with negative TextLength returns HY090.", "[query][prepare][error]") {
  // Doc: "HY090 - Invalid string or buffer length: The argument TextLength was
  //       less than or equal to 0 but not equal to SQL_NTS."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare is called with a negative text length
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1", -5);

  // Then it should return SQL_ERROR with SQLSTATE HY090
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY090");
}

TEST_CASE("SQLPrepare with zero TextLength returns HY090.", "[query][prepare][error]") {
  // Doc: "HY090 - Invalid string or buffer length: The argument TextLength was
  //       less than or equal to 0 but not equal to SQL_NTS."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare is called with zero text length
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1", 0);

  // Then it should return SQL_ERROR with SQLSTATE HY090
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY090");
}

TEST_CASE("SQLPrepare with empty SQL string returns HY090.", "[query][prepare][error]") {
  // Doc: "HY090 - Invalid string or buffer length"
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare is called with an empty SQL string
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"", SQL_NTS);

  // Then it should return SQL_ERROR with SQLSTATE HY090
  REQUIRE(ret == SQL_ERROR);
  // TODO: Check why this is different on Windows
  UNIX_ONLY { CHECK(get_sqlstate(stmt) == "HY090"); }
  WINDOWS_ONLY {
    auto sqlstate = get_sqlstate(stmt);
    CHECK((sqlstate == "HY090" || sqlstate == "HY000"));
  }
}

TEST_CASE("SQLPrepare with invalid SQL syntax returns 42000.", "[query][prepare][error]") {
  // Doc: "42000 - Syntax error or access violation: The SQL statement contained
  //       in *StatementText was not valid."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare is called with invalid SQL syntax
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"NOT VALID SQL SYNTAX!!!", SQL_NTS);

  // Then it should return SQL_ERROR with SQLSTATE 42000
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "42000");
}

TEST_CASE("SQLPrepare with cursor already open returns 24000.", "[query][prepare][error]") {
  // Doc: "24000 - Invalid cursor state: A cursor was open on the StatementHandle."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#diagnostics

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // And a query has been executed leaving a cursor open
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1 AS value", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // When SQLPrepare is called while the cursor is still open
  ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 2 AS new_value", SQL_NTS);

  // Then it should return SQL_ERROR with SQLSTATE 24000
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "24000");
}

// =============================================================================
// DDL / DML Edge Cases
// =============================================================================

TEST_CASE("DDL via SQLPrepare + SQLExecute.", "[query][prepare]") {
  // Doc: "SQLPrepare prepares an SQL string for execution."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  auto stmt = conn.createStatement();

  // When a CREATE TABLE is prepared and executed
  SQLRETURN ret =
      SQLPrepare(stmt.getHandle(), (SQLCHAR*)"CREATE TABLE prep_ddl_test (id INT, name VARCHAR(100))", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the table should exist
  auto verify_stmt = conn.execute("SELECT COUNT(*) FROM prep_ddl_test");
  SQLRETURN fetch_ret = SQLFetch(verify_stmt.getHandle());
  REQUIRE_ODBC(fetch_ret, verify_stmt);

  SQLINTEGER count = -1;
  SQLLEN indicator = 0;
  fetch_ret = SQLGetData(verify_stmt.getHandle(), 1, SQL_C_LONG, &count, sizeof(count), &indicator);
  REQUIRE_ODBC(fetch_ret, verify_stmt);
  CHECK(count == 0);
}

TEST_CASE("DML returning SQL_NO_DATA via SQLPrepare + SQLExecute.", "[query][prepare]") {
  // Doc: "SQL_NO_DATA is returned if the SQL statement was an UPDATE, INSERT,
  //       or DELETE statement that did not affect any rows."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecute-function#returns

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE prep_dml_nodata (id INT)");
  auto stmt = conn.createStatement();

  // When a DELETE that affects no rows is prepared and executed
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"DELETE FROM prep_dml_nodata WHERE 1=0", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());

  // Then it should return SQL_NO_DATA
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("INSERT via SQLPrepare + SQLExecute with verify.", "[query][prepare]") {
  // Doc: "SQLPrepare prepares an SQL string for execution."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlprepare-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE prep_insert_test (id INT, name VARCHAR(100))");

  // When an INSERT is prepared with bound parameters and executed
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"INSERT INTO prep_insert_test VALUES (?, ?)", SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQLINTEGER id_val = 1;
    SQLLEN id_len = sizeof(id_val);
    ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &id_val, sizeof(id_val),
                           &id_len);
    REQUIRE_ODBC(ret, stmt);

    std::string name_val = "test_name";
    SQLLEN name_len = name_val.size();
    ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, name_val.size(), 0,
                           (SQLCHAR*)name_val.c_str(), name_val.size(), &name_len);
    REQUIRE_ODBC(ret, stmt);

    ret = SQLExecute(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
  }

  // Then the inserted row should be retrievable
  auto verify_stmt = conn.execute_fetch("SELECT id, name FROM prep_insert_test");
  CHECK(get_data<SQL_C_LONG>(verify_stmt, 1) == 1);
  CHECK(get_data<SQL_C_CHAR>(verify_stmt, 2) == "test_name");
}
