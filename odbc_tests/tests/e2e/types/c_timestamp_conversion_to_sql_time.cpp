// ODBC E2E: SQL_C_TYPE_TIMESTAMP bound via SQLBindParameter to ODBC 3.x
// SQL_TYPE_TIME.
//
// Per ODBC Appendix D ("C to SQL: Timestamp"), binding a timestamp to a
// TIME target silently discards the date fields. The Snowflake TIME type
// supports nanosecond precision, so the fractional-seconds portion is
// preserved (mirrors the SnowflakeTime::read_odbc behavior).

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void bind_timestamp_and_execute(StatementHandleWrapper& stmt, SQL_TIMESTAMP_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIME, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// SQL_C_TYPE_TIMESTAMP → TIME — happy paths
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME and discard date component",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_TYPE_TIMESTAMP 2026-04-13 14:30:45 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 14, 30, 45, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then only the time is preserved (date silently discarded)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 14);
  CHECK(result.minute == 30);
  CHECK(result.second == 45);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP with nanos to SQL_TYPE_TIME and preserve nanos",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column (Snowflake TIME supports nanosecond precision)
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp carries a half-second fractional component
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 14, 30, 45, 500000000};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the fractional seconds round-trip via the textual representation
  // (Snowflake TIME -> SQL_C_CHAR trims trailing zeros, so .500000000 -> .5)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "14:30:45.5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind midnight SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp's time portion is 00:00:00
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 0, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the stored value is the zero time
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 0);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind end-of-day SQL_C_TYPE_TIMESTAMP to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When 23:59:59 on any date is bound
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {2026, 4, 13, 23, 59, 59, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then the upper-bound time is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 23);
  CHECK(result.minute == 59);
  CHECK(result.second == 59);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP at epoch date to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When the timestamp date is the Unix epoch but the time is non-zero
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT val = {1970, 1, 1, 12, 0, 0, 0};
  SQLLEN ind = sizeof(val);
  bind_timestamp_and_execute(stmt, val, ind);

  // Then only the time matters; the epoch date is irrelevant
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIME_STRUCT result = get_data<SQL_C_TYPE_TIME>(fetch_stmt, 1);
  CHECK(result.hour == 12);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIMESTAMP with NULL indicator to SQL_TYPE_TIME",
                 "[c_timestamp][conversion][sql_time]") {
  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_TYPE_TIMESTAMP is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIMESTAMP, SQL_TYPE_TIME, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_TIME>(fetch_stmt, 1) == std::nullopt);
}
