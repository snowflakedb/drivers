#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

TEST_CASE("should bind SQL_C_NUMERIC to SQL_VARCHAR.", "[query][bind_parameter][c_numeric_to_varchar]") {
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

TEST_CASE("should bind negative SQL_C_NUMERIC with scale to SQL_VARCHAR.",
          "[query][bind_parameter][c_numeric_to_varchar]") {
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
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "-123.45");
}

TEST_CASE("should bind SQL_C_NUMERIC with negative scale to SQL_VARCHAR.",
          "[query][bind_parameter][c_numeric_to_varchar]") {
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
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "12300");
}
