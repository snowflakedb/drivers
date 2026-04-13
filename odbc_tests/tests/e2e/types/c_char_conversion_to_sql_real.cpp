#include <cmath>
#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_CHAR float string to SQL_DOUBLE", "[c_char][conversion][sql_real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col DOUBLE)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "3.14";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(3.14, 0.001));
}

TEST_CASE("should bind SQL_C_CHAR integer string to SQL_REAL", "[c_char][conversion][sql_real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col DOUBLE)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "100";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_REAL, 0, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(100.0, 0.001));
}

TEST_CASE("should bind SQL_C_WCHAR float string to SQL_DOUBLE", "[c_char][conversion][sql_real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col DOUBLE)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'2', '.', '7', '1', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_DOUBLE, 0, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(2.71, 0.001));
}

TEST_CASE("should bind SQL_C_CHAR with NULL indicator to SQL_DOUBLE", "[c_char][conversion][sql_real]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col DOUBLE)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DOUBLE, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_DOUBLE>(fetch_stmt, 1) == std::nullopt);
}
