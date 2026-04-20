#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE to SQL_VARCHAR and read back",
                 "[c_float][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(100))");

  // When SQL_C_DOUBLE 3.14 is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLDOUBLE val = 3.14;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the string representation contains 3.14
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  std::string s = get_data<SQL_C_CHAR>(fetch_stmt, 1);
  CHECK(s.find("3.14") != std::string::npos);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_FLOAT to SQL_VARCHAR and read back",
                 "[c_float][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(100))");

  // When SQL_C_FLOAT 42.0 is bound to SQL_VARCHAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLREAL val = 42.0f;
  SQLLEN ind = 0;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_FLOAT, SQL_VARCHAR, 100, 0, &val, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the string representation contains 42
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  std::string s = get_data<SQL_C_CHAR>(fetch_stmt, 1);
  CHECK(s.find("42") != std::string::npos);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_DOUBLE with NULL indicator to SQL_VARCHAR",
                 "[c_float][conversion][sql_string]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(100))");

  // When SQL_C_DOUBLE is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_VARCHAR, 100, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}
