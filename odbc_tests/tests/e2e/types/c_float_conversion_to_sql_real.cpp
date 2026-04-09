// ODBC E2E: SQL_C_FLOAT / SQL_C_DOUBLE bound via SQLBindParameter to SQL real types (SQL_REAL, SQL_FLOAT, SQL_DOUBLE)

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cfloat>
#include <cmath>
#include <optional>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_DOUBLE to SQL_DOUBLE and read back", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When A double value is bound with SQL_C_DOUBLE and SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 3.14;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back correctly as SQL_C_DOUBLE
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(3.14));
}

TEST_CASE("should bind SQL_C_FLOAT to SQL_REAL and read back", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When A float value is bound with SQL_C_FLOAT and SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = 1.5f;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_REAL, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should be read back as SQL_C_DOUBLE
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(1.5).epsilon(1e-6));
}

TEST_CASE("should bind SQL_C_DOUBLE negative zero", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When Negative zero is bound as SQL_C_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = -0.0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched value should be floating-point zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  double read = get_data<SQL_C_DOUBLE>(fetch_stmt, 1);
  CHECK(read == Catch::Approx(0.0));
  CHECK(std::fpclassify(read) == FP_ZERO);
}

TEST_CASE("should bind SQL_C_DOUBLE large value", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When A large double near DBL_MAX is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 1.7e308;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should round-trip within floating-point precision
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(1.7e308).epsilon(1e-12));
}

TEST_CASE("should bind SQL_C_DOUBLE small value", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When A very small positive double is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 2.2e-308;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should round-trip within floating-point precision
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  OLD_DRIVER_ONLY("BD#36") { CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == 0.0); }
  NEW_DRIVER_ONLY("BD#36") { CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(2.2e-308).epsilon(1e-12)); }
}

TEST_CASE("should bind SQL_C_FLOAT max value", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When FLT_MAX is bound with SQL_C_FLOAT and SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = FLT_MAX;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_REAL, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The value should read back matching FLT_MAX
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(static_cast<double>(FLT_MAX)).epsilon(1e-6));
}

TEST_CASE("should bind SQL_C_DOUBLE with NULL indicator", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When SQL_NULL_DATA is used for the SQL_C_DOUBLE parameter
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The column value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_DOUBLE>(fetch_stmt, 1) == std::nullopt);
}

TEST_CASE("should bind SQL_C_DOUBLE zero", "[c_float][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When Zero is bound as SQL_C_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 0.0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The fetched value should be zero
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_DOUBLE>(fetch_stmt, 1) == Catch::Approx(0.0));
}
