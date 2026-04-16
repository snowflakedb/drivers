#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_floating_point.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_BIT one to SQL_DOUBLE", "[c_bit][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When SQL_C_BIT 1 is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 1.0
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(1.0, 0.001));
}

TEST_CASE("should bind SQL_C_BIT zero to SQL_DOUBLE", "[c_bit][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When SQL_C_BIT 0 is bound to SQL_DOUBLE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR val = 0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_DOUBLE, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 0.0
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(0.0, 0.001));
}

TEST_CASE("should bind SQL_C_BIT one to SQL_REAL", "[c_bit][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When SQL_C_BIT 1 is bound to SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR val = 1;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_REAL, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 1.0
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(1.0, 0.001));
}

TEST_CASE("should bind SQL_C_BIT zero to SQL_REAL", "[c_bit][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When SQL_C_BIT 0 is bound to SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLCHAR val = 0;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_REAL, 0, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 0.0
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_THAT(get_data<SQL_C_DOUBLE>(fetch_stmt, 1), Catch::Matchers::WithinAbs(0.0, 0.001));
}

TEST_CASE("should bind SQL_C_BIT with NULL indicator to SQL_REAL", "[c_bit][conversion][sql_real]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col FLOAT)");

  // When SQL_C_BIT is bound with SQL_NULL_DATA to SQL_REAL and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BIT, SQL_REAL, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_DOUBLE>(fetch_stmt, 1) == std::nullopt);
}
