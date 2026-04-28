// ODBC E2E: SQL_C_TYPE_TIMESTAMP bound via SQLBindParameter to ODBC 3.x
// SQL_TYPE_DATE.
//
// Per ODBC Appendix D ("C to SQL: Timestamp"), binding a timestamp to a
// DATE target silently discards the time fields. These tests verify the
// round-trip preserves Y/M/D and that a non-zero time portion does NOT
// cause the date to roll forward.

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void bind_timestamp_and_execute(StatementHandleWrapper& stmt, SQL_TIMESTAMP_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_DATE, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// SQL_C_TYPE_TIMESTAMP → DATE — happy paths
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_DATE and discard time component",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 14, 30, 45, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then only the date is preserved (time silently discarded)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind midnight SQL_C_TYPE_TIMESTAMP to SQL_TYPE_DATE without info loss",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the timestamp's time portion is already 00:00:00
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 0, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the date round-trips exactly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP with nanos to SQL_TYPE_DATE and discard nanos",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the timestamp carries a non-zero nanosecond fraction
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 12, 0, 0, 999999999};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the nanos are silently dropped along with the time
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should bind end-of-day SQL_C_TYPE_TIMESTAMP to SQL_TYPE_DATE without rolling forward",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When 23:59:59 on 2026-04-13 is bound; the spec says we discard the time,
  // we must NOT round up to 2026-04-14
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 23, 59, 59, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the stored date is the original date, not the next day
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind leap-day SQL_C_TYPE_TIMESTAMP to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When the leap day 2024-02-29 14:30:00 is bound
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2024, 2, 29, 14, 30, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the leap date is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_DATE_STRUCT result = get_data<SQL_C_TYPE_DATE>(fetch_stmt, 1);
  CHECK(result.year == 2024);
  CHECK(result.month == 2);
  CHECK(result.day == 29);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_DATE",
                 "[c_timestamp][conversion][sql_date]") {
  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_DATE, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_DATE>(fetch_stmt, 1) == std::nullopt);
}
