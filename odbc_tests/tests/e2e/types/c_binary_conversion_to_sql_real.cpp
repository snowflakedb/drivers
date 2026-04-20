// ODBC E2E: SQL_C_BINARY bound via SQLBindParameter to SQL real types (SQL_DOUBLE, SQL_REAL, SQL_FLOAT)

#include <cmath>
#include <limits>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY f64 to SQL_DOUBLE and read back",
                 "[c_binary][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When An 8-byte binary buffer containing an f64 is bound as SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 3.14;
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_DOUBLE, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(3.14));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY f32 to SQL_REAL and read back",
                 "[c_binary][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A 4-byte binary buffer containing an f32 is bound as SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = 2.5f;
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_REAL, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(2.5).epsilon(1e-6));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should accept SQL_C_BINARY NaN for SQL_DOUBLE",
                 "[c_binary][conversion][sql_real]") {
  // Per MS ODBC "C to SQL: Binary", the only validation for SQL_C_BINARY ->
  // SQL_DOUBLE is the length-equals check (8 bytes). NaN is a valid IEEE-754
  // value that Snowflake FLOAT columns accept, so the bind and insert must
  // succeed and the value must round-trip.
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When An 8-byte binary buffer containing NaN is bound as SQL_DOUBLE
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = std::numeric_limits<SQLDOUBLE>::quiet_NaN();
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_DOUBLE, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The NaN value should round-trip back to the client as NaN
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  double fetched = get_data<SQL_C_DOUBLE>(fetch_stmt, 1);
  CHECK(std::isnan(fetched));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should accept SQL_C_BINARY infinity for SQL_REAL",
                 "[c_binary][conversion][sql_real]") {
  // Per MS ODBC "C to SQL: Binary", the only validation for SQL_C_BINARY ->
  // SQL_REAL is the length-equals check (4 bytes). +Infinity is a valid
  // IEEE-754 value that Snowflake FLOAT columns accept, so the bind and
  // insert must succeed and the value must round-trip.
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A 4-byte binary buffer containing infinity is bound as SQL_REAL
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = std::numeric_limits<SQLREAL>::infinity();
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_REAL, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The infinity value should round-trip back to the client as +Infinity
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  double fetched = get_data<SQL_C_DOUBLE>(fetch_stmt, 1);
  CHECK(std::isinf(fetched));
  CHECK(fetched > 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for SQL_DOUBLE",
                 "[c_binary][conversion][sql_real]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col FLOAT)");

  // When A 3-byte binary buffer is bound as SQL_DOUBLE
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[3] = {1, 2, 3};
  SQLLEN ind = sizeof(buf);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_DOUBLE, 0, 0, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}
