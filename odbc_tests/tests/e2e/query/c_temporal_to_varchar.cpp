#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

TEST_CASE("should bind SQL_C_TYPE_TIMESTAMP to SQL_VARCHAR.", "[query][bind_parameter][c_temporal_to_varchar]") {
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

TEST_CASE("should bind SQL_C_TYPE_TIMESTAMP with fraction to SQL_VARCHAR.",
          "[query][bind_parameter][c_temporal_to_varchar]") {
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

TEST_CASE("should bind SQL_C_TYPE_DATE to SQL_VARCHAR.", "[query][bind_parameter][c_temporal_to_varchar]") {
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

TEST_CASE("should bind SQL_C_TYPE_TIME to SQL_VARCHAR.", "[query][bind_parameter][c_temporal_to_varchar]") {
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
