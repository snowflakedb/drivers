// ODBC E2E: SQL_C_TYPE_TIME bound via SQLBindParameter to a TIME target.
//
// Per ODBC Appendix G ("Driver Guidelines for Backward Compatibility"),
// the ODBC 3.x time code SQL_TYPE_TIME (92) and its ODBC 2.x predecessor
// SQL_TIME (10) must be accepted as identical at the SQLBindParameter
// boundary. Each TEST_CASE below is parametrized over both spellings
// using Catch2 GENERATE so the alias contract is pinned for every
// scenario.

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to TIME target and read back",
                 "[c_time][conversion][sql_time]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_TYPE_TIME 14:30:45 is bound to the TIME target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT val = {};
  val.hour = 14;
  val.minute = 30;
  val.second = 45;
  SQLLEN ind = sizeof(val);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, sql_type, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 14:30:45
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  auto time = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME with NULL indicator to TIME target",
                 "[c_time][conversion][sql_time]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_TYPE_TIME is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, sql_type, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}
