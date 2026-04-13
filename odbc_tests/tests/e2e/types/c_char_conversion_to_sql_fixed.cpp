#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_CHAR integer string to SQL_INTEGER", "[c_char][conversion][sql_fixed]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "42";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_INTEGER, 0, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 42);
}

TEST_CASE("should bind SQL_C_CHAR negative integer string to SQL_BIGINT", "[c_char][conversion][sql_fixed]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "-9999999999";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BIGINT, 0, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -9999999999LL);
}

TEST_CASE("should bind SQL_C_CHAR decimal string to SQL_DECIMAL", "[c_char][conversion][sql_fixed]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER(10,2))");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "3.14";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_DECIMAL, 10, 2, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "3.14");
}

TEST_CASE("should bind SQL_C_WCHAR integer string to SQL_INTEGER", "[c_char][conversion][sql_fixed]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'7', '7', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, SQL_INTEGER, 0, 0, val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 77);
}

TEST_CASE("should bind SQL_C_CHAR with NULL indicator to SQL_INTEGER", "[c_char][conversion][sql_fixed]") {
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_INTEGER, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_SBIGINT>(fetch_stmt, 1).has_value());
}
