#include <cstring>
#include <string>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

// =============================================================================
// Tests for SQLBindParameter based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function
// E2E round-trip tests: bind parameter -> execute -> fetch -> verify result
// =============================================================================

// =============================================================================
// Integer Types
// =============================================================================

template <typename ParamT, typename ResultT>
void verify_integer_roundtrip(const StatementHandleWrapper& stmt, SQLSMALLINT c_type, SQLSMALLINT sql_type,
                              ParamT value, SQLSMALLINT result_c_type) {
  ParamT param = value;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, sql_type, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ResultT result = 0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, result_c_type, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == static_cast<ResultT>(value));
}

TEST_CASE("SQLBindParameter binds integer types and round-trips through SELECT.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then each integer C type should round-trip the bound value
  SECTION("SQL_C_SLONG") {
    verify_integer_roundtrip<SQLINTEGER, SQLINTEGER>(stmt, SQL_C_SLONG, SQL_INTEGER, 42, SQL_C_SLONG);
  }
  SECTION("SQL_C_SHORT") {
    verify_integer_roundtrip<SQLSMALLINT, SQLINTEGER>(stmt, SQL_C_SHORT, SQL_SMALLINT, 12345, SQL_C_SLONG);
  }
  SECTION("SQL_C_SBIGINT") {
    verify_integer_roundtrip<SQLBIGINT, SQLBIGINT>(stmt, SQL_C_SBIGINT, SQL_BIGINT, 9223372036854775807LL,
                                                   SQL_C_SBIGINT);
  }
  SECTION("SQL_C_STINYINT") {
    verify_integer_roundtrip<SQLSCHAR, SQLINTEGER>(stmt, SQL_C_STINYINT, SQL_TINYINT, 127, SQL_C_SLONG);
  }
  SECTION("negative SQL_C_SLONG") {
    verify_integer_roundtrip<SQLINTEGER, SQLINTEGER>(stmt, SQL_C_SLONG, SQL_INTEGER, -42, SQL_C_SLONG);
  }
  SECTION("SQL_C_UTINYINT") {
    verify_integer_roundtrip<SQLCHAR, SQLINTEGER>(stmt, SQL_C_UTINYINT, SQL_TINYINT, 255, SQL_C_SLONG);
  }
}

// =============================================================================
// Float / Double Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_DOUBLE and round-trips with precision.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_DOUBLE parameter is bound with value 3.14159265358979
  SQLDOUBLE param = 3.14159265358979;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return the double value with precision
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLDOUBLE result = 0.0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == Catch::Approx(3.14159265358979).epsilon(1e-10));
}

TEST_CASE("SQLBindParameter binds SQL_C_FLOAT and round-trips through SELECT.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_FLOAT parameter is bound with value 2.5
  SQLREAL param = 2.5f;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_REAL, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return the float value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLDOUBLE result = 0.0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DOUBLE, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == Catch::Approx(2.5).epsilon(1e-5));
}

// =============================================================================
// Decimal / Numeric Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_CHAR to SQL_DECIMAL and round-trips through INSERT/SELECT.",
          "[query][bind_parameter]") {
  // Doc: "For SQL_DECIMAL or SQL_NUMERIC, ColumnSize is the defined precision."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function#columnsize

  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a DECIMAL column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_decimal_test (val DECIMAL(10,2))");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_decimal_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_CHAR parameter is bound with a decimal string value
  char param[] = "12345.67";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 2, param, sizeof(param),
                         &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the value should return 12345.67
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_decimal_test");
  REQUIRE(get_data<SQL_C_CHAR>(select_stmt, 1) == "12345.67");
}

TEST_CASE("SQLBindParameter binds SQL_C_CHAR to SQL_NUMERIC and round-trips through SELECT.",
          "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared with SQL_NUMERIC parameter type
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  char param[] = "99999";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_NUMERIC, 10, 0, param, sizeof(param),
                         &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return the value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 99999);
}

// =============================================================================
// String Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_CHAR with SQL_NTS and round-trips through SELECT.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_CHAR parameter is bound with null-terminated string
  char param[] = "hello world";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(param), 0, param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return the string
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char result[256] = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(result) == "hello world");
}

TEST_CASE("SQLBindParameter binds SQL_C_CHAR with explicit length.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_CHAR parameter is bound with explicit length
  char param[] = "hello world";
  SQLLEN indicator = 5;  // Only "hello"
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 5, 0, param, sizeof(param),
                         &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return the substring defined by the length
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char result[256] = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(result) == "hello");
}

TEST_CASE("SQLBindParameter binds SQL_C_CHAR with empty string.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_CHAR parameter is bound with an empty string
  char param[] = "";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 0, 0, param, sizeof(param),
                         &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return an empty string
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char result[256] = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(result) == "");
}

TEST_CASE("SQLBindParameter binds SQL_C_WCHAR (UTF-16) string and round-trips through SELECT.",
          "[query][bind_parameter]") {
  // Doc: "SQLBindParameter supports binding to a Unicode C data type, even if
  //       the underlying driver does not support Unicode data."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function#summary

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_WCHAR parameter is bound with a UTF-16 string
  std::u16string param = u"wide hello";
  SQLLEN indicator = param.size() * sizeof(char16_t);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_WVARCHAR, param.size(), 0,
                         (SQLWCHAR*)param.c_str(), indicator, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return the string
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char result[256] = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(result) == "wide hello");
}

// =============================================================================
// Boolean Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_BIT true and round-trips through SELECT.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_BIT parameter is bound with value 1
  SQLCHAR param = 1;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return true
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLCHAR result = 0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BIT, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 1);
}

TEST_CASE("SQLBindParameter binds SQL_C_BIT false and round-trips through SELECT.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_BIT parameter is bound with value 0
  SQLCHAR param = 0;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_BIT, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return false
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLCHAR result = 1;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BIT, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 0);
}

// =============================================================================
// Binary Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_BINARY and round-trips through INSERT/SELECT.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a BINARY column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_binary_test (val BINARY(10))");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_binary_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_BINARY parameter is bound with binary data
  unsigned char param[] = {0x48, 0x65, 0x6C, 0x6C, 0x6F};  // "Hello"
  SQLLEN indicator = sizeof(param);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, sizeof(param), 0, param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the data should return the original binary content
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_binary_test");
  unsigned char result[64] = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_BINARY, result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result_ind == 5);
  REQUIRE(memcmp(result, param, 5) == 0);
}

// =============================================================================
// Date Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_TYPE_DATE struct and round-trips through INSERT/SELECT.",
          "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a DATE column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_date_test (val DATE)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_date_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_TYPE_DATE parameter is bound with date 2025-03-15
  SQL_DATE_STRUCT param = {};
  param.year = 2025;
  param.month = 3;
  param.day = 15;
  SQLLEN indicator = sizeof(param);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_DATE, 0, 0, &param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the date should return 2025-03-15
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_date_test");
  SQL_DATE_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_DATE, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.year == 2025);
  REQUIRE(result.month == 3);
  REQUIRE(result.day == 15);
}

TEST_CASE("SQLBindParameter binds SQL_C_TYPE_DATE with epoch date.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a DATE column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_date_epoch_test (val DATE)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_date_epoch_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_TYPE_DATE parameter is bound with date 1970-01-01
  SQL_DATE_STRUCT param = {};
  param.year = 1970;
  param.month = 1;
  param.day = 1;
  SQLLEN indicator = sizeof(param);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_DATE, 0, 0, &param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the date should return 1970-01-01
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_date_epoch_test");
  SQL_DATE_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_DATE, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.year == 1970);
  REQUIRE(result.month == 1);
  REQUIRE(result.day == 1);
}

TEST_CASE("SQLBindParameter binds date as SQL_C_CHAR string to SQL_TYPE_DATE.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a DATE column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_date_str_test (val DATE)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_date_str_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And a date string is bound as SQL_C_CHAR to SQL_TYPE_DATE
  char param[] = "2025-03-15";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_TYPE_DATE, 10, 0, param, sizeof(param),
                         &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the date should return 2025-03-15
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_date_str_test");
  SQL_DATE_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_DATE, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.year == 2025);
  REQUIRE(result.month == 3);
  REQUIRE(result.day == 15);
}

// =============================================================================
// Time Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_TYPE_TIME struct and round-trips through INSERT/SELECT.",
          "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a TIME column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_time_test (val TIME)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_time_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_TYPE_TIME parameter is bound with time 10:30:45
  SQL_TIME_STRUCT param = {};
  param.hour = 10;
  param.minute = 30;
  param.second = 45;
  SQLLEN indicator = sizeof(param);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIME, 0, 0, &param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the time should return 10:30:45
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_time_test");
  SQL_TIME_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_TIME, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.hour == 10);
  REQUIRE(result.minute == 30);
  REQUIRE(result.second == 45);
}

TEST_CASE("SQLBindParameter binds SQL_C_TYPE_TIME with midnight.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a TIME column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_time_midnight_test (val TIME)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_time_midnight_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_TYPE_TIME parameter is bound with time 00:00:00
  SQL_TIME_STRUCT param = {};
  param.hour = 0;
  param.minute = 0;
  param.second = 0;
  SQLLEN indicator = sizeof(param);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIME, 0, 0, &param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the time should return 00:00:00
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_time_midnight_test");
  SQL_TIME_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_TIME, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.hour == 0);
  REQUIRE(result.minute == 0);
  REQUIRE(result.second == 0);
}

TEST_CASE("SQLBindParameter binds time as SQL_C_CHAR string to SQL_TYPE_TIME.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with a TIME column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_time_str_test (val TIME)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_time_str_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And a time string is bound as SQL_C_CHAR to SQL_TYPE_TIME
  char param[] = "14:30:00";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_TYPE_TIME, 8, 0, param, sizeof(param),
                         &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the time should return 14:30:00
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_time_str_test");
  SQL_TIME_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_TIME, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.hour == 14);
  REQUIRE(result.minute == 30);
  REQUIRE(result.second == 0);
}

// =============================================================================
// Timestamp Types
// =============================================================================

TEST_CASE("SQLBindParameter binds SQL_C_TYPE_TIMESTAMP to TIMESTAMP_NTZ and round-trips.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And the session timezone is set to UTC
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // And a temporary table with a TIMESTAMP_NTZ column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_ts_test (val TIMESTAMP_NTZ(9))");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_ts_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_TYPE_TIMESTAMP parameter is bound with fractional seconds
  SQL_TIMESTAMP_STRUCT param = {};
  param.year = 2025;
  param.month = 6;
  param.day = 15;
  param.hour = 10;
  param.minute = 30;
  param.second = 45;
  param.fraction = 123456000;  // nanoseconds
  SQLLEN indicator = sizeof(param);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 29, 9, &param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the timestamp should return the expected components
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_ts_test");
  SQL_TIMESTAMP_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_TIMESTAMP, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.year == 2025);
  REQUIRE(result.month == 6);
  REQUIRE(result.day == 15);
  REQUIRE(result.hour == 10);
  REQUIRE(result.minute == 30);
  REQUIRE(result.second == 45);
  REQUIRE(result.fraction == 123456000);
}

TEST_CASE("SQLBindParameter binds timestamp as SQL_C_CHAR string.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And the session timezone is set to UTC
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // And a temporary table with a TIMESTAMP_NTZ column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_ts_str_test (val TIMESTAMP_NTZ)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_ts_str_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And an SQL_C_CHAR parameter is bound with a timestamp string
  char param[] = "2025-06-15 10:30:45";
  SQLLEN indicator = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_TYPE_TIMESTAMP, 19, 0, param,
                         sizeof(param), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the INSERT is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then selecting the timestamp should return the expected value
  auto select_stmt = conn.execute_fetch("SELECT val FROM bind_ts_str_test");
  SQL_TIMESTAMP_STRUCT result = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_TYPE_TIMESTAMP, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result.year == 2025);
  REQUIRE(result.month == 6);
  REQUIRE(result.day == 15);
  REQUIRE(result.hour == 10);
  REQUIRE(result.minute == 30);
  REQUIRE(result.second == 45);
}

// =============================================================================
// NULL Handling
// =============================================================================

TEST_CASE("SQLBindParameter binds NULL via SQL_NULL_DATA indicator.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // And a parameter is bound with SQL_NULL_DATA indicator
  SQLLEN indicator = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, nullptr, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return NULL
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 999;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result_ind == SQL_NULL_DATA);
}

TEST_CASE("SQLBindParameter mixes NULL and non-NULL in sequential executions.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;

  // And a temporary table with an INTEGER column exists
  auto schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TEMPORARY TABLE bind_null_mix_test (val INTEGER)");

  auto stmt = conn.createStatement();

  // When a parameterized INSERT is prepared
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO bind_null_mix_test VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER param = 0;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And a non-NULL integer is inserted followed by a NULL and another non-NULL
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

  // Then selecting all rows should return the expected values with one NULL
  auto select_stmt = conn.execute("SELECT val FROM bind_null_mix_test ORDER BY val NULLS FIRST");

  // First row: NULL
  ret = SQLFetch(select_stmt.getHandle());
  REQUIRE_ODBC(ret, select_stmt);
  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result_ind == SQL_NULL_DATA);

  // Second row: 100
  ret = SQLFetch(select_stmt.getHandle());
  REQUIRE_ODBC(ret, select_stmt);
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result == 100);

  // Third row: 200
  ret = SQLFetch(select_stmt.getHandle());
  REQUIRE_ODBC(ret, select_stmt);
  ret = SQLGetData(select_stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, select_stmt);
  REQUIRE(result == 200);
}

// =============================================================================
// Multi-Parameter and Rebinding
// =============================================================================

TEST_CASE("SQLBindParameter binds multiple typed parameters in one statement.", "[query][bind_parameter]") {
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
  REQUIRE_ODBC(ret, stmt);

  char str_param[] = "hello";
  SQLLEN str_ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(str_param), 0, str_param,
                         sizeof(str_param), &str_ind);
  REQUIRE_ODBC(ret, stmt);

  // Then executing and fetching should return both values
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER int_result = 0;
  SQLLEN int_result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &int_result, sizeof(int_result), &int_result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(int_result == 42);

  char str_result[256] = {};
  SQLLEN str_result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_CHAR, str_result, sizeof(str_result), &str_result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(str_result) == "hello");
}

TEST_CASE("SQLBindParameter re-executes prepared statement with changed bound value.", "[query][bind_parameter]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared and bound with value 10
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER param = 10;
  SQLLEN indicator = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And the statement is executed and the result verified
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 10);

  // And the cursor is closed and the bound variable changed to 20
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  param = 20;

  // Then re-executing should return 20
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 20);
}

TEST_CASE("SQLFreeStmt SQL_RESET_PARAMS clears bindings and allows re-binding.", "[query][bind_parameter]") {
  // Doc: "A variable remains bound until it is rebound or until all parameters are
  //       unbound by calling SQLFreeStmt with the SQL_RESET_PARAMS option."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameterized SELECT is prepared and an integer is bound
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER int_param = 42;
  SQLLEN int_ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &int_param, 0, &int_ind);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(get_data<SQL_C_LONG>(stmt, 1) == 42);

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
  REQUIRE_ODBC(ret, stmt);

  // Then re-executing should return the new string value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char result[256] = {};
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(result) == "rebound");
}

TEST_CASE("SQLBindParameter rebinds parameter to different type without SQL_RESET_PARAMS.", "[query][bind_parameter]") {
  // Doc: "Bindings remain in effect until the application calls SQLBindParameter again,
  //       calls SQLFreeStmt with the SQL_RESET_PARAMS option..."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlbindparameter-function#comments

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
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER int_result = 0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &int_result, sizeof(int_result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(int_result == 42);

  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And the same parameter is rebound as a string without calling SQL_RESET_PARAMS
  char str_param[] = "rebound_no_reset";
  SQLLEN str_ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, strlen(str_param), 0, str_param,
                         sizeof(str_param), &str_ind);
  REQUIRE_ODBC(ret, stmt);

  // Then re-executing should return the new string value
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  char str_result[256] = {};
  SQLLEN str_result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, str_result, sizeof(str_result), &str_result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(std::string(str_result) == "rebound_no_reset");
}

TEST_CASE("SQLExecDirect with bound parameter executes without SQLPrepare.", "[query][bind_parameter]") {
  // Doc: "If the statement contains parameter markers, the application uses SQLBindParameter
  //       to bind each parameter before passing the SQL statement to SQLExecDirect."
  // https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlexecdirect-function#comments

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a parameter is bound before calling SQLExecDirect
  SQLINTEGER param = 77;
  SQLLEN indicator = 0;
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_SLONG, SQL_INTEGER, 0, 0, &param, 0, &indicator);
  REQUIRE_ODBC(ret, stmt);

  // And SQLExecDirect is called with a parameterized query
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the bound parameter value should be returned
  SQLINTEGER result = 0;
  SQLLEN result_ind = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_SLONG, &result, sizeof(result), &result_ind);
  REQUIRE_ODBC(ret, stmt);
  REQUIRE(result == 77);
}
