// ODBC E2E: SQL_C_BINARY bound via SQLBindParameter to SQL integer types (SQL_INTEGER, SQL_BIGINT, SQL_SMALLINT,
// SQL_TINYINT)

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY i32 to SQL_INTEGER and read back",
                 "[c_binary][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When A 4-byte binary buffer containing an i32 is bound as SQL_INTEGER and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = 42;
  SQLLEN ind = sizeof(val);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_INTEGER, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 42);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY i64 to SQL_BIGINT and read back",
                 "[c_binary][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When An 8-byte binary buffer containing an i64 is bound as SQL_BIGINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLBIGINT val = 9999999999LL;
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BIGINT, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 9999999999LL);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY i16 to SQL_SMALLINT and read back",
                 "[c_binary][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When A 2-byte binary buffer containing an i16 is bound as SQL_SMALLINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSMALLINT val = -7;
  SQLLEN ind = sizeof(val);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_SMALLINT, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -7);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY i8 to SQL_TINYINT and read back",
                 "[c_binary][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When A 1-byte binary buffer containing an i8 is bound as SQL_TINYINT and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLSCHAR val = 127;
  SQLLEN ind = sizeof(val);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TINYINT, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == 127);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY negative i32 to SQL_INTEGER and read back",
                 "[c_binary][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When A 4-byte binary buffer containing a negative i32 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER val = -100;
  SQLLEN ind = sizeof(val);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_INTEGER, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -100);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for SQL_INTEGER",
                 "[c_binary][conversion][sql_fixed]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When A 3-byte binary buffer is bound as SQL_INTEGER
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[3] = {1, 2, 3};
  SQLLEN ind = sizeof(buf);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_INTEGER, 0, 0, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}
