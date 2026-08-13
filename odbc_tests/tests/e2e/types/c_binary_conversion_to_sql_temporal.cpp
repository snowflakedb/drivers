// ODBC E2E: SQL_C_BINARY bound via SQLBindParameter to SQL temporal types
// (DATE, TIME, TIMESTAMP).
// The binary buffer is interpreted as the raw bytes of the corresponding C temporal struct.
//
// Per ODBC Appendix G ("Driver Guidelines for Backward Compatibility"), the ODBC 3.x
// codes SQL_TYPE_DATE/TIME/TIMESTAMP (91/92/93) and their ODBC 2.x predecessors
// SQL_DATE/TIME/TIMESTAMP (9/10/11) must be accepted as identical at the
// SQLBindParameter boundary, so every case is parametrized over both spellings
// with Catch2 GENERATE.

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_BINARY → SQL_TYPE_DATE
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY date struct to a DATE target and read back",
                 "[c_binary][conversion][sql_temporal]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When A 6-byte binary buffer containing a SQL_DATE_STRUCT is bound as the DATE target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT ds = {2025, 3, 26};
  SQLLEN ind = sizeof(ds);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 0, 0, &ds, sizeof(ds), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The date should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2025-03-26");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for a DATE target",
                 "[c_binary][conversion][sql_temporal]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When A 4-byte buffer is bound as the DATE target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[4] = {};
  SQLLEN ind = sizeof(buf);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 0, 0, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}

// ============================================================================
// SQL_C_BINARY → SQL_TYPE_TIME
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY time struct to a TIME target and read back",
                 "[c_binary][conversion][sql_temporal]") {
  SKIP_OLD_DRIVER("BD#46", "Old driver does not support SQL_C_BINARY as source for SQL_TYPE_TIME");
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When A 6-byte binary buffer containing a SQL_TIME_STRUCT is bound as the TIME target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT ts = {14, 30, 45};
  SQLLEN ind = sizeof(ts);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 0, 0, &ts, sizeof(ts), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The time should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "14:30:45");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for a TIME target",
                 "[c_binary][conversion][sql_temporal]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When A 4-byte buffer is bound as the TIME target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[4] = {};
  SQLLEN ind = sizeof(buf);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 0, 0, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}

// ============================================================================
// SQL_C_BINARY → SQL_TYPE_TIMESTAMP
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY timestamp struct to a TIMESTAMP target and read back",
                 "[c_binary][conversion][sql_temporal]") {
  SKIP_OLD_DRIVER("BD#46", "Old driver does not support SQL_C_BINARY as source for SQL_TYPE_TIMESTAMP");
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // NTZ binds as wall-clock TEXT and the server applies the session TIMEZONE
  // offset; pin UTC so the stored value equals the bound wall-clock.
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When A 16-byte binary buffer containing a SQL_TIMESTAMP_STRUCT is bound as the TIMESTAMP target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT tss = {2025, 3, 26, 14, 30, 45, 0};
  SQLLEN ind = sizeof(tss);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 0, 0, &tss, sizeof(tss), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // TIMESTAMP_NTZ -> SQL_C_CHAR strips the fractional part when the
  // nanoseconds component is zero.
  //
  // Then The timestamp should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2025-03-26 14:30:45");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY timestamp with fractional seconds",
                 "[c_binary][conversion][sql_temporal]") {
  SKIP_OLD_DRIVER("BD#46", "Old driver does not support SQL_C_BINARY as source for SQL_TYPE_TIMESTAMP");
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // NTZ binds as wall-clock TEXT and the server applies the session TIMEZONE
  // offset; pin UTC so the stored value equals the bound wall-clock.
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When A timestamp struct with 500ms fractional part is bound
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT tss = {2025, 1, 1, 0, 0, 0, 500000000};
  SQLLEN ind = sizeof(tss);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 0, 9, &tss, sizeof(tss), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // TIMESTAMP_NTZ -> SQL_C_CHAR trims trailing zeros from the nanosecond
  // component, so .500000000 becomes .5.
  //
  // Then The timestamp with fraction should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2025-01-01 00:00:00.5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for a TIMESTAMP target",
                 "[c_binary][conversion][sql_temporal]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When An 8-byte buffer is bound as the TIMESTAMP target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[8] = {};
  SQLLEN ind = sizeof(buf);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, sql_type, 0, 0, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}
