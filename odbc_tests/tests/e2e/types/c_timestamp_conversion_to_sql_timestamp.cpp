#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIMESTAMP and read back",
          "[c_timestamp][conversion][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {};
  val.year = 2026;
  val.month = 4;
  val.day = 13;
  val.hour = 14;
  val.minute = 30;
  val.second = 45;
  val.fraction = 0;
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 0, 0, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the value contains the date and time components
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  std::string result = get_data<SQL_C_CHAR>(fetch_stmt, 1);
  CHECK(result.find("2026-04-13") != std::string::npos);
  CHECK(result.find("14:30:45") != std::string::npos);
}

TEST_CASE("should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_TIMESTAMP",
          "[c_timestamp][conversion][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIMESTAMP, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1) == std::nullopt);
}
