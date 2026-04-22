#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to SQL_TYPE_DATE and read back",
                 "[c_date][conversion][sql_date]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_DATE and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT val = {};
  val.year = 2026;
  val.month = 4;
  val.day = 13;
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_DATE, 0, 0, &val, sizeof(val),
                         &ind);
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

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE with NULL indicator to SQL_TYPE_DATE",
                 "[c_date][conversion][sql_date]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_DATE is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_DATE, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_DATE>(fetch_stmt, 1) == std::nullopt);
}
