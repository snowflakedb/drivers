// ODBC E2E: SQL_C_TYPE_DATE bound via SQLBindParameter to a DATE target.
//
// Per ODBC Appendix G ("Driver Guidelines for Backward Compatibility"),
// the ODBC 3.x date code SQL_TYPE_DATE (91) and its ODBC 2.x predecessor
// SQL_DATE (9) must be accepted as identical at the SQLBindParameter
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

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to DATE target and read back",
                 "[c_date][conversion][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_DATE 2026-04-13 is bound to the DATE target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT val = {};
  val.year = 2026;
  val.month = 4;
  val.day = 13;
  SQLLEN ind = sizeof(val);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, sql_type, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value is read back as 2026-04-13
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE with NULL indicator to DATE target",
                 "[c_date][conversion][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_DATE is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, sql_type, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_DATE>(fetch_stmt, 1) == std::nullopt);
}
