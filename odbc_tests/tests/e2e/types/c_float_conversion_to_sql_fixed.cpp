// ODBC C float types (SQL_C_FLOAT, SQL_C_DOUBLE) bound to fixed SQL types
// (SQL_INTEGER, SQL_BIGINT, SQL_DECIMAL) via SQLBindParameter.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <optional>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_DOUBLE to SQL_INTEGER and read back", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  // When A double value is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 42.0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_INTEGER, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 42);
}

TEST_CASE("should bind SQL_C_FLOAT to SQL_INTEGER and read back", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  // When A float value is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = -100.0f;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_INTEGER, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -100);
}

TEST_CASE("should bind SQL_C_DOUBLE with fraction to SQL_INTEGER truncates", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  // When A fractional double is bound to INTEGER and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 42.99;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_INTEGER, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The stored value should truncate toward zero when read as integer
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 42);
}

TEST_CASE("should bind SQL_C_FLOAT zero to SQL_INTEGER", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  // When Float zero is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = 0.0f;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_INTEGER, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as integer zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 0);
}

TEST_CASE("should bind SQL_C_DOUBLE negative to SQL_BIGINT", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  // When A negative double is bound to BIGINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = -123456.0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_BIGINT, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly as 64-bit integer
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -123456);
}

TEST_CASE("should bind SQL_C_DOUBLE to SQL_DECIMAL and read back", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER(10,2))");

  // When A double is bound to DECIMAL(10,2) and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 3.14;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DECIMAL, 10, 2, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should match as a character string
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  std::string s = get_data<SQL_C_CHAR>(fetch_stmt, 1);
  CHECK(s == "3.14");
}

TEST_CASE("should reject SQL_C_DOUBLE overflow into NUMBER(3,0)", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER(3,0))");

  // When A double value exceeding the column precision is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 99999.0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_INTEGER, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the server rejects the value with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

TEST_CASE("should bind SQL_C_DOUBLE with NULL indicator to SQL_INTEGER", "[c_float][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  // When NULL is bound via SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_INTEGER, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The column should fetch as NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_SBIGINT>(fetch_stmt, 1) == std::nullopt);
}
