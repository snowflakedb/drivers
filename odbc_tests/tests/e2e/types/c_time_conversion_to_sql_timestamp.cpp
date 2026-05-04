// ODBC E2E: SQL_C_TYPE_TIME bound via SQLBindParameter to ODBC 3.x
// SQL_TYPE_TIMESTAMP across all three Snowflake variants.
//
// Per ODBC Appendix D ("C to SQL: Time"): "The date fields of the
// timestamp structure are set to the current date, and the fractional
// seconds portion of the timestamp is set to zero." The "current date"
// here is the driver host's local date at bind time.
//
// To stay deterministic across midnight rollover, the date assertion
// captures the local date both before and after the bind and accepts
// any value within that window.

#include <ctime>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

struct LocalYmd {
  int year;
  int month;
  int day;
};

LocalYmd local_today() {
  std::time_t t = std::time(nullptr);
  std::tm tm{};
#ifdef _WIN32
  localtime_s(&tm, &t);
#else
  localtime_r(&t, &tm);
#endif
  return {tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday};
}

bool ymd_lte(const LocalYmd& a, const SQL_TIMESTAMP_STRUCT& b) {
  if (a.year != b.year) return a.year < b.year;
  if (a.month != b.month) return a.month < b.month;
  return a.day <= b.day;
}

bool ymd_gte(const LocalYmd& a, const SQL_TIMESTAMP_STRUCT& b) {
  if (a.year != b.year) return a.year > b.year;
  if (a.month != b.month) return a.month > b.month;
  return a.day >= b.day;
}

void bind_time_and_execute(StatementHandleWrapper& stmt, SQL_TIME_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIMESTAMP, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

SQLRETURN bind_time_and_try_execute(StatementHandleWrapper& stmt, SQL_TIME_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIMESTAMP, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  return SQLExecute(stmt.getHandle());
}

}  // namespace

// ============================================================================
// SQL_C_TYPE_TIME → TIMESTAMP_NTZ
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to TIMESTAMP_NTZ with current local date",
                 "[c_time][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_NTZ column
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_TYPE_TIME 14:30:45 is bound to SQL_TYPE_TIMESTAMP and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT val = {14, 30, 45};
  SQLLEN ind = sizeof(val);
  LocalYmd today_before = local_today();
  bind_time_and_execute(stmt, val, ind);
  LocalYmd today_after = local_today();

  // Then the time round-trips exactly the fraction is zero and the date falls within the local clock window at bind
  // time
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.hour == 14);
  CHECK(result.minute == 30);
  CHECK(result.second == 45);
  CHECK(result.fraction == 0);
  INFO("date " << result.year << "-" << result.month << "-" << result.day << " not within [" << today_before.year << "-"
               << today_before.month << "-" << today_before.day << ", " << today_after.year << "-" << today_after.month
               << "-" << today_after.day << "]");
  CHECK(ymd_lte(today_before, result));
  CHECK(ymd_gte(today_after, result));
}

// ============================================================================
// SQL_C_TYPE_TIME → TIMESTAMP_LTZ
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to TIMESTAMP_LTZ with current local date",
                 "[c_time][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_LTZ column with a known session timezone
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_LTZ)");

  // When SQL_C_TYPE_TIME 14:30:45 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT val = {14, 30, 45};
  SQLLEN ind = sizeof(val);
  bind_time_and_execute(stmt, val, ind);

  // Then the bind succeeds and the time component round-trips
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.hour == 14);
  CHECK(result.minute == 30);
  CHECK(result.second == 45);
  CHECK(result.fraction == 0);
}

// ============================================================================
// SQL_C_TYPE_TIME → TIMESTAMP_TZ
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME to TIMESTAMP_TZ with current local date",
                 "[c_time][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_TZ column with a known session timezone
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_TZ)");

  // When SQL_C_TYPE_TIME 14:30:45 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT val = {14, 30, 45};
  SQLLEN ind = sizeof(val);
  bind_time_and_execute(stmt, val, ind);

  // Then the bind succeeds and the time component round-trips
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.hour == 14);
  CHECK(result.minute == 30);
  CHECK(result.second == 45);
  CHECK(result.fraction == 0);
}

// ============================================================================
// NULL indicator
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_TIME with NULL indicator to SQL_TYPE_TIMESTAMP",
                 "[c_time][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_NTZ column
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_TYPE_TIME is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_TIME, SQL_TYPE_TIMESTAMP, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1) == std::nullopt);
}

// ============================================================================
// Invalid-struct-field rejection — SQLSTATE 22007 (Invalid datetime format)
//
// Per ODBC Appendix D ("C to SQL: Time"), a SQL_C_TYPE_TIME struct whose
// fields are outside their legal range (hour not in 0..23, minute or
// second not in 0..59) must surface SQL_ERROR with SQLSTATE 22007.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIME with hour=24 bound to SQL_TYPE_TIMESTAMP",
                 "[c_time][conversion][sql_timestamp][invalid]") {
  // Given a TIMESTAMP_NTZ column
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When the time carries hour=24 which is out of the legal range
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT val = {24, 0, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_time_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIME with minute=60 bound to SQL_TYPE_TIMESTAMP",
                 "[c_time][conversion][sql_timestamp][invalid]") {
  // Given a TIMESTAMP_NTZ column
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When the time carries minute=60 which is out of the legal range
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT val = {12, 60, 0};
  SQLLEN ind = sizeof(val);
  ret = bind_time_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_TYPE_TIME with second=60 bound to SQL_TYPE_TIMESTAMP",
                 "[c_time][conversion][sql_timestamp][invalid]") {
  // Given a TIMESTAMP_NTZ column
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When the time carries second=60 and Snowflake does not honor leap seconds
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT val = {12, 0, 60};
  SQLLEN ind = sizeof(val);
  ret = bind_time_and_try_execute(stmt, val, ind);

  // Then SQLExecute fails with SQLSTATE 22007
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22007"));
}
