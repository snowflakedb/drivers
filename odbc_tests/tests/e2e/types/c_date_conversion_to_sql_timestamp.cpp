// ODBC E2E: SQL_C_TYPE_DATE bound via SQLBindParameter to ODBC 3.x
// SQL_TYPE_TIMESTAMP across all three Snowflake variants (TIMESTAMP_NTZ,
// TIMESTAMP_LTZ, TIMESTAMP_TZ).
//
// Per ODBC Appendix D ("C to SQL: Date"), binding a date to a timestamp
// target sets the time portion to 00:00:00.000000000. These tests verify
// the round-trip preserves Y/M/D and produces a zeroed time portion.

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

void bind_date_and_execute(StatementHandleWrapper& stmt, SQL_DATE_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_TIMESTAMP, 0, 0,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// SQL_C_TYPE_DATE → TIMESTAMP_NTZ
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to TIMESTAMP_NTZ at midnight",
                 "[c_date][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_NTZ column
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_TIMESTAMP and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT val = {2026, 4, 13};
  SQLLEN ind = sizeof(val);
  bind_date_and_execute(stmt, val, ind);

  // Then the stored value has the bound date and a zeroed time component
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
  CHECK(result.hour == 0);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
  CHECK(result.fraction == 0);
}

// ============================================================================
// SQL_C_TYPE_DATE → TIMESTAMP_LTZ
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to TIMESTAMP_LTZ at midnight UTC",
                 "[c_date][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_LTZ column with a known session timezone
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_LTZ)");

  // When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_TIMESTAMP and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT val = {2026, 4, 13};
  SQLLEN ind = sizeof(val);
  bind_date_and_execute(stmt, val, ind);

  // Then the stored value has the bound date and midnight UTC
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
  CHECK(result.hour == 0);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
}

// ============================================================================
// SQL_C_TYPE_DATE → TIMESTAMP_TZ
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE to TIMESTAMP_TZ at midnight UTC",
                 "[c_date][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_TZ column with a known session timezone
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_TZ)");

  // When SQL_C_TYPE_DATE 2026-04-13 is bound to SQL_TYPE_TIMESTAMP and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT val = {2026, 4, 13};
  SQLLEN ind = sizeof(val);
  bind_date_and_execute(stmt, val, ind);

  // Then the stored value has the bound date and midnight UTC
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.year == 2026);
  CHECK(result.month == 4);
  CHECK(result.day == 13);
  CHECK(result.hour == 0);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
}

// ============================================================================
// Edge cases
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE leap day 2024-02-29 to SQL_TYPE_TIMESTAMP",
                 "[c_date][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_NTZ column with a known session timezone
  // (TIMEZONE=UTC is required because the legacy 3.16.0 driver routes
  // DATE → TIMESTAMP through a TZ-aware path; without pinning TZ the
  // legacy driver would shift the day by the session's UTC offset.)
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When the leap day 2024-02-29 is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT val = {2024, 2, 29};
  SQLLEN ind = sizeof(val);
  bind_date_and_execute(stmt, val, ind);

  // Then the leap day is preserved
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.year == 2024);
  CHECK(result.month == 2);
  CHECK(result.day == 29);
  CHECK(result.hour == 0);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE epoch 1970-01-01 to SQL_TYPE_TIMESTAMP",
                 "[c_date][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_NTZ column with a known session timezone (see leap-day
  // test above for rationale)
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When the Unix epoch date is bound and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT val = {1970, 1, 1};
  SQLLEN ind = sizeof(val);
  bind_date_and_execute(stmt, val, ind);

  // Then the epoch date is preserved at midnight
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  SQL_TIMESTAMP_STRUCT result = get_data<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1);
  CHECK(result.year == 1970);
  CHECK(result.month == 1);
  CHECK(result.day == 1);
  CHECK(result.hour == 0);
  CHECK(result.minute == 0);
  CHECK(result.second == 0);
  CHECK(result.fraction == 0);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_TYPE_DATE with NULL indicator to SQL_TYPE_TIMESTAMP",
                 "[c_date][conversion][sql_timestamp]") {
  // Given a TIMESTAMP_NTZ column
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_TYPE_DATE is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_TYPE_DATE, SQL_TYPE_TIMESTAMP, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data_optional<SQL_C_TYPE_TIMESTAMP>(fetch_stmt, 1) == std::nullopt);
}
