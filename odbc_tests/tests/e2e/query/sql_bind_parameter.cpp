#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// =============================================================================
// ODBC SQLBindParameter spec-compliance tests
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function
// =============================================================================

// =============================================================================
// Error Codes — Diagnostics table
// =============================================================================

TEST_CASE("should return SQL_INVALID_HANDLE for null statement handle.", "[query][bind_parameter][error]") {
  // Given No statement handle exists
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;

  // When SQLBindParameter is called with SQL_NULL_HSTMT
  SQLRETURN ret =
      SQLBindParameter(SQL_NULL_HSTMT, 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);

  // Then SQL_INVALID_HANDLE should be returned
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("should return 07009 when ParameterNumber is zero.", "[query][bind_parameter][error]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindParameter is called with ParameterNumber 0
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 0, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);

  // Then SQL_ERROR with SQLSTATE 07009 should be returned
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("07009"));
}

TEST_CASE("should return HY003 for invalid C data type.", "[query][bind_parameter][error]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindParameter is called with invalid ValueType 9999
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, 9999, SQL_INTEGER, 0, 0, &param, 0, &indicator);

  // Then SQL_ERROR with SQLSTATE HY003 should be returned
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY003"));
}

TEST_CASE("should return HY004 for invalid SQL data type.", "[query][bind_parameter][error]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindParameter is called with invalid ParameterType 8888
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, 8888, 0, 0, &param, 0, &indicator);

  // Then SQL_ERROR with SQLSTATE HY004 should be returned
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY004"));
}

TEST_CASE("should return HY105 for invalid InputOutputType.", "[query][bind_parameter][error]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindParameter is called with invalid InputOutputType 999
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, 999, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);

  // Then SQL_ERROR with SQLSTATE HY105 should be returned
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY105"));
}

TEST_CASE("should return HY009 when both value and indicator pointers are null.", "[query][bind_parameter][error]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindParameter is called with null ParameterValuePtr and null StrLen_or_IndPtr
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, nullptr, 0, nullptr);

  // Then SQL_ERROR with SQLSTATE HY009 should be returned
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY009"));
}

TEST_CASE("should return HY090 for negative BufferLength.", "[query][bind_parameter][error]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindParameter is called with BufferLength -1
  char param[] = "test";
  SQLLEN indicator = SQL_NTS;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 4, 0, param, -1, &indicator);

  // Then SQL_ERROR with SQLSTATE HY090 should be returned
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY090"));
}

TEST_CASE("should return HY104 for invalid precision or scale.", "[query][bind_parameter][error]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLBindParameter is called with negative DecimalDigits
  char param[] = "1.23";
  SQLLEN indicator = SQL_NTS;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 5, -1, param,
                                   sizeof(param), &indicator);

  // Then the new driver rejects with HY104, the old driver accepts it
  NEW_DRIVER_ONLY("BD#28") {
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY104"));
  }
  OLD_DRIVER_ONLY("BD#28") { REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded()); }
}

// TODO: Add HY021 (inconsistent descriptor) and HYC00 (unsupported conversion)
// tests when those validations are implemented.

// =============================================================================
// API Behavior — Binding lifecycle
// =============================================================================

TEST_CASE("should execute via SQLExecDirect with pre-bound parameter.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameter is bound before calling SQLExecDirect
  SQLINTEGER param = 77;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // And SQLExecDirect is called with a parameterized query
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the bound parameter value should be returned
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 77);
}

TEST_CASE("should replace binding when same ParameterNumber is rebound.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And parameter 1 is bound to value 111
  SQLINTEGER param1 = 111;
  SQLLEN ind1 = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param1, 0, &ind1);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // And parameter 1 is rebound to value 222
  SQLINTEGER param2 = 222;
  SQLLEN ind2 = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param2, 0, &ind2);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then executing should return the latest bound value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 222);
}

// TODO: Add 07002 (SQL_RESET_PARAMS) test in PR #566 once auto-IPD is implemented (BD#29).

TEST_CASE("should reflect changed bound variable on re-execution.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared and bound to a variable
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER param = 10;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // And first execution returns 10
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 10);
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And the bound variable is changed to 20 without rebinding
  param = 20;

  // Then re-executing should return the updated value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 20);
}

TEST_CASE("should bind multiple parameters to a single statement.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a SELECT with two parameter markers is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ?, ?"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an integer and a string parameter are bound
  SQLINTEGER int_param = 42;
  SQLLEN int_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &int_param, 0, &int_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  char str_param[] = "hello";
  SQLLEN str_ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(str_param), 0, str_param,
                         sizeof(str_param), &str_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then executing and fetching should return both values
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 42);
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "hello");
}

TEST_CASE("should rebind parameter to different type without SQL_RESET_PARAMS.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an integer parameter is bound and executed
  SQLINTEGER int_param = 42;
  SQLLEN int_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &int_param, 0, &int_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 42);
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And the same parameter is rebound as a string without calling SQL_RESET_PARAMS
  char str_param[] = "rebound";
  SQLLEN str_ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(str_param), 0, str_param,
                         sizeof(str_param), &str_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then re-executing should return the new string value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "rebound");
}

TEST_CASE("should bind NULL via SQL_NULL_DATA indicator.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared with a NULL-indicating parameter
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN indicator = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, nullptr, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then executing and fetching should return NULL
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  auto result = get_data_optional<SQL_C_LONG>(stmt, 1);
  CHECK(!result.has_value());
}

TEST_CASE("should alternate NULL and non-NULL across sequential executions.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_null_seq (val INTEGER)");
  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared with a bound integer
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_null_seq VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER param = 0;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // And rows are inserted: 100, NULL, 200
  param = 100;
  indicator = 0;
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE_ODBC(ret, stmt);

  indicator = SQL_NULL_DATA;
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE_ODBC(ret, stmt);

  param = 200;
  indicator = 0;
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting all rows should return 100, NULL, 200
  auto select_stmt = conn.execute("SELECT val FROM bind_null_seq ORDER BY val NULLS FIRST");
  ret = SQLFetch(select_stmt.getHandle());
  REQUIRE_ODBC(ret, select_stmt);
  auto row1 = get_data_optional<SQL_C_LONG>(select_stmt, 1);
  CHECK(!row1.has_value());

  ret = SQLFetch(select_stmt.getHandle());
  REQUIRE_ODBC(ret, select_stmt);
  CHECK(get_data<SQL_C_LONG>(select_stmt, 1) == 100);

  ret = SQLFetch(select_stmt.getHandle());
  REQUIRE_ODBC(ret, select_stmt);
  CHECK(get_data<SQL_C_LONG>(select_stmt, 1) == 200);
}

TEST_CASE("should allow rebinding after SQL_RESET_PARAMS.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared and an integer is bound
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER int_param = 42;
  SQLLEN int_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &int_param, 0, &int_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 42);
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And all parameter bindings are reset
  ret = SQLFreeStmt(stmt.getHandle(), SQL_RESET_PARAMS);
  REQUIRE_ODBC(ret, stmt);

  // And a new string parameter is bound to the same parameter position
  char str_param[] = "rebound";
  SQLLEN str_ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(str_param), 0, str_param,
                         sizeof(str_param), &str_ind);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then re-executing should return the new string value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "rebound");
}

// TODO: Add APD/IPD descriptor integration tests in PR #566:
// - APD fields populated after bind
// - IPD fields populated after bind
// - SQLNumParams after binding
// - SQLDescribeParam after binding
