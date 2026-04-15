#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

TEST_CASE("should bind SQL_C_CHAR '1' to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "1";
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_CHAR '0' to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "0";
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be FALSE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}

TEST_CASE("should bind SQL_C_WCHAR '1' to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLWCHAR param[] = {'1', 0};
  SQLLEN indicator = 1 * sizeof(SQLWCHAR);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_WCHAR '0' to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLWCHAR param[] = {'0', 0};
  SQLLEN indicator = 1 * sizeof(SQLWCHAR);
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be FALSE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}

TEST_CASE("should bind SQL_C_CHAR 'true' to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver only accepts '0'/'1' strings for SQL_BIT");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "true";
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_CHAR 'false' to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver only accepts '0'/'1' strings for SQL_BIT");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "false";
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be FALSE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 0);
}

TEST_CASE("should bind SQL_C_CHAR numeric '42' to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  SKIP_OLD_DRIVER("BD-35", "Old driver only accepts '0'/'1' strings for SQL_BIT");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  char param[] = "42";
  SQLLEN indicator = SQL_NTS;
  // When the C type value is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE (nonzero numeric string)
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_WCHAR with SQL_NTS to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLWCHAR param[] = {'1', 0};
  SQLLEN indicator = SQL_NTS;
  // When SQL_C_WCHAR is bound with SQL_NTS indicator as SQL_BIT
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_BIT, 1, 0, param,
                                   sizeof(param), &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be TRUE
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_CHAR NULL to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLLEN indicator = SQL_NULL_DATA;
  // When SQL_C_CHAR with SQL_NULL_DATA is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, nullptr, 0,
                                   &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be NULL
  CHECK(get_data_optional<SQL_C_BIT>(stmt, 1) == std::nullopt);
}

TEST_CASE("should bind SQL_C_WCHAR NULL to SQL_BIT.", "[query][bind_parameter][c_char_to_boolean]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();
  SQLLEN indicator = SQL_NULL_DATA;
  // When SQL_C_WCHAR with SQL_NULL_DATA is bound as SQL_BIT and SELECT ? is executed
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_BIT, 1, 0, nullptr, 0,
                                   &indicator);
  REQUIRE_ODBC_SUCCESS(ret, stmt);
  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT ? AS val"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // Then the result should be NULL
  CHECK(get_data_optional<SQL_C_BIT>(stmt, 1) == std::nullopt);
}
