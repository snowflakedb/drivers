// ODBC E2E: SQL_C_BINARY bound via SQLBindParameter to SQL temporal types (SQL_DATE, SQL_TIME, SQL_TIMESTAMP)
// The binary buffer is interpreted as the raw bytes of the corresponding C struct.

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_BINARY → SQL_TYPE_DATE
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY date struct to SQL_TYPE_DATE and read back",
                 "[c_binary][conversion][sql_temporal]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When A 6-byte binary buffer containing a SQL_DATE_STRUCT is bound as SQL_TYPE_DATE
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_DATE_STRUCT ds = {2025, 3, 26};
  SQLLEN ind = sizeof(ds);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TYPE_DATE, 0, 0, &ds, sizeof(ds), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The date should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2025-03-26");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for SQL_TYPE_DATE",
                 "[c_binary][conversion][sql_temporal]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When A 4-byte buffer is bound as SQL_TYPE_DATE
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[4] = {};
  SQLLEN ind = sizeof(buf);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TYPE_DATE, 0, 0, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}

// ============================================================================
// SQL_C_BINARY → SQL_TYPE_TIME
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY time struct to SQL_TYPE_TIME and read back",
                 "[c_binary][conversion][sql_temporal]") {
  SKIP_OLD_DRIVER("BD#46", "Old driver does not support SQL_C_BINARY as source for SQL_TYPE_TIME");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When A 6-byte binary buffer containing a SQL_TIME_STRUCT is bound as SQL_TYPE_TIME
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIME_STRUCT ts = {14, 30, 45};
  SQLLEN ind = sizeof(ts);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TYPE_TIME, 0, 0, &ts, sizeof(ts), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then The time should be read back correctly
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "14:30:45");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for SQL_TYPE_TIME",
                 "[c_binary][conversion][sql_temporal]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When A 4-byte buffer is bound as SQL_TYPE_TIME
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[4] = {};
  SQLLEN ind = sizeof(buf);
  ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TYPE_TIME, 0, 0, buf, sizeof(buf), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}

// ============================================================================
// SQL_C_BINARY → SQL_TYPE_TIMESTAMP
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_BINARY timestamp struct to SQL_TYPE_TIMESTAMP and read back",
                 "[c_binary][conversion][sql_temporal]") {
  SKIP_OLD_DRIVER("BD#46", "Old driver does not support SQL_C_BINARY as source for SQL_TYPE_TIMESTAMP");
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When A 16-byte binary buffer containing a SQL_TIMESTAMP_STRUCT is bound as SQL_TYPE_TIMESTAMP
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT tss = {2025, 3, 26, 14, 30, 45, 0};
  SQLLEN ind = sizeof(tss);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TYPE_TIMESTAMP, 0, 0, &tss,
                         sizeof(tss), &ind);
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
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When A timestamp struct with 500ms fractional part is bound
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_TIMESTAMP_STRUCT tss = {2025, 1, 1, 0, 0, 0, 500000000};
  SQLLEN ind = sizeof(tss);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TYPE_TIMESTAMP, 0, 9, &tss,
                         sizeof(tss), &ind);
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

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BINARY with wrong size for SQL_TYPE_TIMESTAMP",
                 "[c_binary][conversion][sql_temporal]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When An 8-byte buffer is bound as SQL_TYPE_TIMESTAMP
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  unsigned char buf[8] = {};
  SQLLEN ind = sizeof(buf);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_TYPE_TIMESTAMP, 0, 0, buf, sizeof(buf),
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then The execution should fail with SQLSTATE 22003
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("22003"));
}
