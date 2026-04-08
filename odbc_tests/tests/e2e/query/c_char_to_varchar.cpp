#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

TEST_CASE("should bind SQL_C_CHAR to SQL_VARCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "hello";
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");
}

TEST_CASE("should bind SQL_C_WCHAR to SQL_VARCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLWCHAR param[] = {'h', 'e', 'l', 'l', 'o', 0};
  SQLLEN indicator = 5 * sizeof(SQLWCHAR);
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 100, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");
}

TEST_CASE("should bind SQL_C_WCHAR with SQL_NTS to SQL_VARCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLWCHAR param[] = {'h', 'e', 'l', 'l', 'o', 0};
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound with SQL_NTS indicator and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_VARCHAR, 100, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");
}

TEST_CASE("should bind SQL_C_DEFAULT to SQL_VARCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "hello";
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound as a string SQL type and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DEFAULT, SQL_VARCHAR, 100, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be the expected string
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");
}

TEST_CASE("should bind SQL_C_SLONG to SQL_CHAR.", "[query][bind_parameter][c_char_to_varchar]") {
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

TEST_CASE("should bind SQL_C_SLONG to SQL_LONGVARCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
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

TEST_CASE("should bind SQL_C_SLONG to SQL_WCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
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

TEST_CASE("should bind SQL_C_SLONG to SQL_WVARCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
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

TEST_CASE("should bind SQL_C_SLONG to SQL_WLONGVARCHAR.", "[query][bind_parameter][c_char_to_varchar]") {
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
