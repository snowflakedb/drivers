#include <algorithm>
#include <cctype>
#include <climits>
#include <cstring>
#include <limits>
#include <string>

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
  NEW_DRIVER_ONLY("BD#26") {
    REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("HY104"));
  }
  OLD_DRIVER_ONLY("BD#26") { REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::Succeeded()); }
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

TEST_CASE("should fail with 07002 after SQL_RESET_PARAMS clears bindings.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared and executed successfully
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER param = 42;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And all parameter bindings are reset
  ret = SQLFreeStmt(stmt.getHandle(), SQL_RESET_PARAMS);
  REQUIRE_ODBC(ret, stmt);

  // Then re-executing should fail with SQLSTATE 07002
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("07002"));
}

TEST_CASE("should fail with 07002 when parameter bindings have a gap.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a query with 3 parameter markers is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ?, ?, ?"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And only parameters 1 and 3 are bound (gap at parameter 2)
  SQLINTEGER p1 = 1, p3 = 3;
  SQLLEN ind1 = 0, ind3 = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &p1, 0, &ind1);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &p3, 0, &ind3);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then executing should fail with SQLSTATE 07002
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("07002"));
}

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

// =============================================================================
// APD/IPD Descriptor Integration
// =============================================================================

TEST_CASE("should populate APD descriptor fields after SQLBindParameter.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a char parameter is bound with explicit buffer length and indicator
  char param[] = "hello";
  SQLLEN indicator = SQL_NTS;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(param), 0,
                                   param, sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then the APD record should reflect all bound fields
  SQLHDESC apd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLSMALLINT concise_type = 0;
  ret = SQLGetDescField(apd, 1, SQL_DESC_CONCISE_TYPE, &concise_type, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(concise_type == SQL_C_CHAR);

  SQLPOINTER data_ptr = nullptr;
  ret = SQLGetDescField(apd, 1, SQL_DESC_DATA_PTR, &data_ptr, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(data_ptr == param);

  SQLLEN octet_length = -1;
  ret = SQLGetDescField(apd, 1, SQL_DESC_OCTET_LENGTH, &octet_length, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(octet_length == sizeof(param));

  SQLLEN* ind_ptr = nullptr;
  ret = SQLGetDescField(apd, 1, SQL_DESC_INDICATOR_PTR, &ind_ptr, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(ind_ptr == &indicator);

  SQLLEN* octet_len_ptr = nullptr;
  ret = SQLGetDescField(apd, 1, SQL_DESC_OCTET_LENGTH_PTR, &octet_len_ptr, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(octet_len_ptr == &indicator);

  // And the APD header should report the correct count
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(count == 1);
}

TEST_CASE("should populate IPD descriptor fields after SQLBindParameter.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a decimal parameter is bound with precision and scale
  char param[] = "123.45";
  SQLLEN indicator = SQL_NTS;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 2, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then the IPD record should reflect all bound fields
  SQLHDESC ipd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_PARAM_DESC, &ipd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLSMALLINT concise_type = 0;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_CONCISE_TYPE, &concise_type, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(concise_type == SQL_DECIMAL);

  SQLSMALLINT param_type = 0;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_PARAMETER_TYPE, &param_type, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(param_type == SQL_PARAM_INPUT);

  SQLSMALLINT precision = -1;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_PRECISION, &precision, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(precision == 10);

  SQLSMALLINT scale = -1;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_SCALE, &scale, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(scale == 2);

  SQLSMALLINT nullable = -1;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_NULLABLE, &nullable, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(nullable == SQL_NULLABLE);

  // And the IPD header should report the correct count
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(ipd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(count == 1);
}

TEST_CASE("should report parameter count via SQLNumParams after binding.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a statement with two parameter markers is prepared and both are bound
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ?, ?"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER p1 = 1, p2 = 2;
  SQLLEN i1 = 0, i2 = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &p1, 0, &i1);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &p2, 0, &i2);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then SQLNumParams should return 2
  SQLSMALLINT num_params = 0;
  ret = SQLNumParams(stmt.getHandle(), &num_params);
  REQUIRE_ODBC(ret, stmt);
  CHECK(num_params == 2);
}

TEST_CASE("should describe bound parameter via SQLDescribeParam.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared and an integer parameter is bound
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER param = 42;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then SQLDescribeParam should return the SQL type information
  SQLSMALLINT data_type = 0;
  SQLULEN param_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLSMALLINT nullable = 0;
  ret = SQLDescribeParam(stmt.getHandle(), 1, &data_type, &param_size, &decimal_digits, &nullable);
  REQUIRE_ODBC(ret, stmt);
  CHECK(data_type == SQL_INTEGER);
}

// =============================================================================
// Descriptor Error Scenarios
// =============================================================================

TEST_CASE("should return SQL_NO_DATA for unbound APD record.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When no parameters are bound and APD record 1 is queried
  SQLHDESC apd = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(apd, 1, SQL_DESC_CONCISE_TYPE, &type, 0, nullptr);

  // Then SQL_NO_DATA should be returned per ODBC spec
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should return SQL_NO_DATA for unbound IPD record.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When no parameters are bound and IPD record 1 is queried
  SQLHDESC ipd = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_PARAM_DESC, &ipd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(ipd, 1, SQL_DESC_CONCISE_TYPE, &type, 0, nullptr);

  // Then the record should not be found (SQL_NO_DATA per spec; old driver returns SQL_ERROR)
  NEW_DRIVER_ONLY("BD#28") { CHECK(ret == SQL_NO_DATA); }
  OLD_DRIVER_ONLY("BD#28") { CHECK(ret == SQL_ERROR); }
}

TEST_CASE("should return error for negative descriptor record number.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When APD is queried with negative record number
  SQLHDESC apd = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLSMALLINT type = -1;
  ret = SQLGetDescField(apd, -1, SQL_DESC_CONCISE_TYPE, &type, 0, nullptr);

  // Then SQL_ERROR should be returned
  CHECK(ret == SQL_ERROR);
}

TEST_CASE("should return error for unknown descriptor field identifier.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in and a parameter is bound
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When APD is queried with an unknown field identifier
  SQLHDESC apd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER dummy = 0;
  ret = SQLGetDescField(apd, 1, 9999, &dummy, 0, nullptr);

  // Then SQL_ERROR should be returned
  CHECK(ret == SQL_ERROR);
}

TEST_CASE("should return error for header-only field on record index greater than zero.",
          "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in and a parameter is bound
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // When APD is queried with SQL_DESC_ARRAY_SIZE (header field) on record 1
  SQLHDESC apd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLULEN array_size = 0;
  ret = SQLGetDescField(apd, 1, SQL_DESC_ARRAY_SIZE, &array_size, 0, nullptr);

  // Then SQL_SUCCESS — per ODBC spec, header fields ignore RecNumber
  CHECK(ret == SQL_SUCCESS);
  CHECK(array_size == 1);
}

TEST_CASE("should report correct APD and IPD count for multiple parameters.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When three parameters are bound
  SQLINTEGER p1 = 1, p2 = 2, p3 = 3;
  SQLLEN i1 = 0, i2 = 0, i3 = 0;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &p1, 0, &i1);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &p2, 0, &i2);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &p3, 0, &i3);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then APD and IPD should both report count 3
  SQLHDESC apd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT apd_count = -1;
  ret = SQLGetDescField(apd, 0, SQL_DESC_COUNT, &apd_count, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(apd_count == 3);

  SQLHDESC ipd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_PARAM_DESC, &ipd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT ipd_count = -1;
  ret = SQLGetDescField(ipd, 0, SQL_DESC_COUNT, &ipd_count, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(ipd_count == 3);
}

TEST_CASE("should reset APD count to zero after SQL_RESET_PARAMS.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameter is bound and then bindings are reset
  SQLINTEGER param = 1;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLFreeStmt(stmt.getHandle(), SQL_RESET_PARAMS);
  REQUIRE_ODBC(ret, stmt);

  // Then APD count should be zero
  SQLHDESC apd = SQL_NULL_HDESC;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT count = -1;
  ret = SQLGetDescField(apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  CHECK(count == 0);
}

TEST_CASE("should report APD count zero when no parameters are bound.", "[query][bind_parameter][descriptor]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When APD header count is queried before any binding
  SQLHDESC apd = SQL_NULL_HDESC;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, &apd, 0, nullptr);
  REQUIRE_ODBC(ret, stmt);

  SQLSMALLINT count = -1;
  ret = SQLGetDescField(apd, 0, SQL_DESC_COUNT, &count, 0, nullptr);
  REQUIRE_ODBC_SUCCESS(ret, stmt);

  // Then count should be 0
  CHECK(count == 0);
}

// =============================================================================
// C type → VARCHAR conversions
// =============================================================================

TEST_CASE("should bind SQL_C_LONG to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

TEST_CASE("should bind SQL_C_SLONG to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

TEST_CASE("should bind negative SQL_C_SLONG to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = -999;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-999");
}

TEST_CASE("should bind SQL_C_ULONG to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLUINTEGER param = 4000000000U;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_ULONG, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "4000000000");
}

TEST_CASE("should bind SQL_C_SSHORT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLSMALLINT param = -7;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SSHORT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-7");
}

TEST_CASE("should bind SQL_C_SHORT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLSMALLINT param = -7;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SHORT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-7");
}

TEST_CASE("should bind SQL_C_USHORT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLUSMALLINT param = 65535;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_USHORT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "65535");
}

TEST_CASE("should bind SQL_C_SBIGINT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLBIGINT param = 9999999999LL;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "9999999999");
}

TEST_CASE("should bind SQL_C_UBIGINT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLUBIGINT param = 1000000000000ULL;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UBIGINT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1000000000000");
}

TEST_CASE("should bind SQL_C_STINYINT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLSCHAR param = -128;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_STINYINT, SQL_VARCHAR, 100, 0, &param, 0,
                                   &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-128");
}

TEST_CASE("should bind SQL_C_TINYINT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLSCHAR param = -128;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TINYINT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-128");
}

TEST_CASE("should bind SQL_C_UTINYINT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLCHAR param = 255;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UTINYINT, SQL_VARCHAR, 100, 0, &param, 0,
                                   &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "255");
}

TEST_CASE("should bind SQL_C_DOUBLE to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = 3.14;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "3.14");
}

TEST_CASE("should bind SQL_C_FLOAT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLREAL param = 1.5f;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1.5");
}

TEST_CASE("should bind SQL_C_BIT true to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLCHAR param = 1;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1");
}

TEST_CASE("should bind SQL_C_BIT false to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLCHAR param = 0;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
}

// =============================================================================
// Boundary values, edge cases, and negative tests
// =============================================================================

TEST_CASE("should bind SQL_C_SLONG INT_MIN to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = INT_MIN;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == std::to_string(INT_MIN));
}

TEST_CASE("should bind SQL_C_SLONG INT_MAX to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = INT_MAX;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == std::to_string(INT_MAX));
}

TEST_CASE("should bind SQL_C_SLONG zero to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 0;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
}

TEST_CASE("should bind SQL_C_SBIGINT LLONG_MIN to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLBIGINT param = LLONG_MIN;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == std::to_string(LLONG_MIN));
}

TEST_CASE("should bind SQL_C_SBIGINT LLONG_MAX to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLBIGINT param = LLONG_MAX;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SBIGINT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == std::to_string(LLONG_MAX));
}

TEST_CASE("should bind SQL_C_UBIGINT ULLONG_MAX to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLUBIGINT param = ULLONG_MAX;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_UBIGINT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == std::to_string(ULLONG_MAX));
}

TEST_CASE("should bind SQL_C_DOUBLE negative zero to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = -0.0;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "0");
}

static std::string to_lower(const std::string& s) {
  std::string lower = s;
  std::transform(lower.begin(), lower.end(), lower.begin(), [](unsigned char c) { return std::tolower(c); });
  return lower;
}

static bool is_nan_str(const std::string& s) { return to_lower(s) == "nan"; }

static bool is_positive_infinity_str(const std::string& s) {
  auto l = to_lower(s);
  return l == "inf" || l == "infinity";
}

static bool is_negative_infinity_str(const std::string& s) {
  auto l = to_lower(s);
  return l == "-inf" || l == "-infinity";
}

TEST_CASE("should bind SQL_C_DOUBLE NaN to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = std::numeric_limits<double>::quiet_NaN();
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(is_nan_str(get_data<SQL_C_CHAR>(stmt, 1)));
}

TEST_CASE("should bind SQL_C_DOUBLE positive infinity to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = std::numeric_limits<double>::infinity();
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(is_positive_infinity_str(get_data<SQL_C_CHAR>(stmt, 1)));
}

TEST_CASE("should bind SQL_C_DOUBLE negative infinity to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLDOUBLE param = -std::numeric_limits<double>::infinity();
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(is_negative_infinity_str(get_data<SQL_C_CHAR>(stmt, 1)));
}

TEST_CASE("should bind SQL_C_FLOAT NaN to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLREAL param = std::numeric_limits<float>::quiet_NaN();
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(is_nan_str(get_data<SQL_C_CHAR>(stmt, 1)));
}

TEST_CASE("should bind SQL_C_FLOAT positive infinity to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLREAL param = std::numeric_limits<float>::infinity();
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(is_positive_infinity_str(get_data<SQL_C_CHAR>(stmt, 1)));
}

TEST_CASE("should bind SQL_C_FLOAT negative infinity to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLREAL param = -std::numeric_limits<float>::infinity();
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(is_negative_infinity_str(get_data<SQL_C_CHAR>(stmt, 1)));
}

TEST_CASE("should bind SQL_C_BIT value > 1 to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  SQLCHAR param = 42;
  SQLLEN indicator = sizeof(param);
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "1");
}

TEST_CASE("should bind SQL_C_DEFAULT to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "hello";
  SQLLEN indicator = SQL_NTS;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_VARCHAR, 100, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");
}

// =============================================================================
// C type → other string SQL types (CHAR, WCHAR, LONGVARCHAR, WVARCHAR, WLONGVARCHAR)
// Verify that all string SQL types correctly route through the same conversion.
// =============================================================================

TEST_CASE("should bind SQL_C_SLONG to SQL_CHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_CHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

TEST_CASE("should bind SQL_C_SLONG to SQL_LONGVARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_LONGVARCHAR, 100, 0, &param,
                                   0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

TEST_CASE("should bind SQL_C_SLONG to SQL_WCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_WCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

TEST_CASE("should bind SQL_C_SLONG to SQL_WVARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_WVARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

TEST_CASE("should bind SQL_C_SLONG to SQL_WLONGVARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLINTEGER param = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_WLONGVARCHAR, 100, 0, &param,
                                   0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

// =============================================================================
// C structured type → VARCHAR conversions
// =============================================================================

TEST_CASE("should bind SQL_C_TYPE_TIMESTAMP to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_TIMESTAMP_STRUCT param = {};
  param.year = 2024;
  param.month = 1;
  param.day = 15;
  param.hour = 10;
  param.minute = 30;
  param.second = 45;
  param.fraction = 0;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_VARCHAR, 100, 0,
                                   &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-01-15 10:30:45");
}

TEST_CASE("should bind SQL_C_TYPE_TIMESTAMP with fraction to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_TIMESTAMP_STRUCT param = {};
  param.year = 2024;
  param.month = 6;
  param.day = 15;
  param.hour = 12;
  param.minute = 0;
  param.second = 0;
  param.fraction = 123456789;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_VARCHAR, 100, 0,
                                   &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-06-15 12:00:00.123456789");
}

TEST_CASE("should bind SQL_C_TYPE_DATE to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_DATE_STRUCT param = {};
  param.year = 2024;
  param.month = 12;
  param.day = 25;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_VARCHAR, 100, 0, &param,
                                   0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "2024-12-25");
}

TEST_CASE("should bind SQL_C_TYPE_TIME to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_TIME_STRUCT param = {};
  param.hour = 14;
  param.minute = 30;
  param.second = 59;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_VARCHAR, 100, 0, &param,
                                   0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "14:30:59");
}

TEST_CASE("should bind SQL_C_NUMERIC to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_NUMERIC_STRUCT param = {};
  param.precision = 10;
  param.scale = 0;
  param.sign = 1;
  memset(param.val, 0, SQL_MAX_NUMERIC_LEN);
  param.val[0] = 42;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "42");
}

TEST_CASE("should bind negative SQL_C_NUMERIC with scale to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_NUMERIC_STRUCT param = {};
  param.precision = 10;
  param.scale = 2;
  param.sign = 0;
  memset(param.val, 0, SQL_MAX_NUMERIC_LEN);
  // 12345 in little-endian: 0x39, 0x30
  param.val[0] = 0x39;
  param.val[1] = 0x30;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  auto result = get_data<SQL_C_CHAR>(stmt, 1);
  NEW_DRIVER_ONLY("BD#33") { CHECK(result == "-123.45"); }
  OLD_DRIVER_ONLY("BD#33") { CHECK(result == "-12345"); }
}

TEST_CASE("should bind SQL_C_NUMERIC with negative scale to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQL_NUMERIC_STRUCT param = {};
  param.precision = 10;
  param.scale = static_cast<SQLSCHAR>(-2);
  param.sign = 1;
  memset(param.val, 0, SQL_MAX_NUMERIC_LEN);
  param.val[0] = 123;
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_VARCHAR, 100, 0, &param, 0, &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  auto result = get_data<SQL_C_CHAR>(stmt, 1);
  NEW_DRIVER_ONLY("BD#33") { CHECK(result == "12300"); }
  OLD_DRIVER_ONLY("BD#33") { CHECK(result == "123"); }
}

TEST_CASE("should bind SQL_C_BINARY to SQL_VARCHAR.", "[query][bind_parameter][c_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  unsigned char param[] = {0xDE, 0xAD, 0xBE, 0xEF};
  SQLLEN indicator = sizeof(param);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_VARCHAR, 100, 0, &param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  NEW_DRIVER_ONLY("BD#34") {
    REQUIRE_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    // Then the result should be the expected string
    CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "deadbeef");
  }
  OLD_DRIVER_ONLY("BD#34") { CHECK(ret == SQL_ERROR); }
}
