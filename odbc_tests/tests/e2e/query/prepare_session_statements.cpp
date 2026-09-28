// ODBC E2E: ALTER SESSION and COMMIT routed through SQLPrepare + SQLExecute.
//
// GS rejects a describeOnly request for these statements with error 000007
// ("Statement provided can not be prepared"), so the driver returns empty
// prepare metadata and lets execute submit the stored SQL without
// describeOnly. Tableau prepares its session setup this way, so these run on
// both drivers to assert parity.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void prepare_and_execute(Connection& conn, const std::string& sql) {
  auto stmt = conn.createStatement();
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS), stmt);
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);
}

}  // namespace

TEST_CASE("should set a session parameter via SQLPrepare + SQLExecute", "[query][prepare][session]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When CLIENT_PREFETCH_THREADS is set to 7 through prepare + execute
  prepare_and_execute(conn, "ALTER SESSION SET CLIENT_PREFETCH_THREADS = 7");

  // Then the session parameter value should be "7"
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'CLIENT_PREFETCH_THREADS'");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == "7");
}

TEST_CASE("should set dollar-quoted QUERY_TAG via SQLPrepare + SQLExecute", "[query][prepare][session]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When QUERY_TAG is set to a dollar-quoted JSON document through prepare + execute
  const std::string tag = R"({"tableau-query-origins": {"workbook": "prepare_e2e"}})";
  prepare_and_execute(conn, "ALTER SESSION SET QUERY_TAG = $$" + tag + "$$");

  // Then the tag round-trips verbatim, dollar quoting and all
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'QUERY_TAG'");
  CHECK(get_data<SQL_C_CHAR>(stmt, 2) == tag);
}

TEST_CASE("should apply a prepared TIMEZONE change to TIMESTAMP_LTZ rendering", "[query][prepare][session][ltz]") {
  // Given Snowflake client is logged in with the session timezone set to UTC
  Connection conn;
  prepare_and_execute(conn, "ALTER SESSION SET TIMEZONE = 'UTC'");
  auto ts_utc =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-06-15 12:00:00'::TIMESTAMP_LTZ"), 1);

  // When the session timezone is changed through prepare + execute
  prepare_and_execute(conn, "ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'");
  auto ts_la =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-06-15 12:00:00'::TIMESTAMP_LTZ"), 1);

  // Then the literal is interpreted in the new zone, so the fetched hour shifts
  CHECK(ts_utc.hour == 12);
  CHECK(ts_la.hour == 19);
}

TEST_CASE("should report no result columns for a prepared session statement", "[query][prepare][session]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When an ALTER SESSION is prepared
  auto stmt = conn.createStatement();
  const std::string sql = "ALTER SESSION SET CLIENT_PREFETCH_THREADS = 5";
  REQUIRE_ODBC(SQLPrepare(stmt.getHandle(), sqlchar(sql.c_str()), SQL_NTS), stmt);

  // Then the describe that was skipped leaves no column or parameter metadata
  SQLSMALLINT num_cols = -1;
  REQUIRE_ODBC(SQLNumResultCols(stmt.getHandle(), &num_cols), stmt);
  CHECK(num_cols == 0);

  SQLSMALLINT num_params = -1;
  REQUIRE_ODBC(SQLNumParams(stmt.getHandle(), &num_params), stmt);
  CHECK(num_params == 0);

  // And the statement still executes
  REQUIRE_ODBC(SQLExecute(stmt.getHandle()), stmt);
}

TEST_CASE("should commit via SQLPrepare + SQLExecute", "[query][prepare][session]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When COMMIT is prepared and executed
  // Then both calls succeed
  prepare_and_execute(conn, "COMMIT");
}
