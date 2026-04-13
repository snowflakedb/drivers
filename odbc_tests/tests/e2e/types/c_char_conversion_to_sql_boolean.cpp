#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_CHAR true string to SQL_BIT", "[c_char][conversion][sql_boolean]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col BOOLEAN)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "1";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_CHAR false string to SQL_BIT", "[c_char][conversion][sql_boolean]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col BOOLEAN)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "0";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 0);
}

TEST_CASE("should bind SQL_C_WCHAR true string to SQL_BIT", "[c_char][conversion][sql_boolean]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col BOOLEAN)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'1', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_BIT, 1, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_BIT>(fetch_stmt, 1) == 1);
}

TEST_CASE("should bind SQL_C_CHAR with NULL indicator to SQL_BIT", "[c_char][conversion][sql_boolean]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col BOOLEAN)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIT, 1, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_BIT>(fetch_stmt, 1) == std::nullopt);
}
