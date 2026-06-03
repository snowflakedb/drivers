// ODBC E2E: SQL_C_CHAR / SQL_C_WCHAR bound via SQLBindParameter to SQL_TYPE_TIME.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types"),
// SQL_C_CHAR and SQL_C_WCHAR are legal source types for SQL_TYPE_TIME. The
// driver parses the character literal as "HH:MM:SS" (optionally with a
// fractional-seconds component) and forwards a typed TIME value; well-formed
// literals round-trip through a TIME column unchanged.
//
// Both drivers accept well-formed literals, so the positive cases run on the
// reference driver too. Malformed literals are rejected by both drivers (the
// new driver surfaces SQLSTATE 07006, the legacy driver 22018), so the negative
// case asserts the error surfaces one of those two conversion SQLSTATEs.
//
// Per ODBC Appendix G ("Driver Guidelines for Backward Compatibility"), the
// ODBC 3.x code SQL_TYPE_TIME (92) and its ODBC 2.x predecessor SQL_TIME (10)
// must be accepted as identical at the SQLBindParameter boundary, so every
// case is parametrized over both spellings with Catch2 GENERATE.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SQL_C_CHAR -> SQL_TYPE_TIME
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR time string to SQL_TYPE_TIME and read back",
                 "[c_char][conversion][sql_time]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_CHAR "14:30:45" is bound as the TIME target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "14:30:45";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the time is read back correctly, both as text and as a typed TIME struct
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "14:30:45");
  auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
  const SQL_TIME_STRUCT ts = get_data<SQL_C_TYPE_TIME>(struct_stmt, 1);
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR time string with fractional seconds to SQL_TYPE_TIME",
                 "[c_char][conversion][sql_time]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given a TIME(9) column that preserves the fractional-seconds component
  conn.execute("CREATE TEMPORARY TABLE t (col TIME(9))");

  // When SQL_C_CHAR "14:30:45.123456789" is bound as the TIME target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "14:30:45.123456789";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 9, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the time with fractional seconds is read back correctly as text, and the
  // typed TIME struct exposes the whole-second components (SQL_TIME_STRUCT carries no
  // fractional-seconds field, so the sub-second part is truncated on that path)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "14:30:45.123456789");
  auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
  const SQL_TIME_STRUCT ts = get_data<SQL_C_TYPE_TIME>(struct_stmt, 1);
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR malformed time string for SQL_TYPE_TIME",
                 "[c_char][conversion][sql_time]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When a non-time SQL_C_CHAR literal is bound as the TIME target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "not-a-time";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the execution fails (new driver: 07006, legacy driver: 22018)
  REQUIRE_THAT(OdbcResult(ret, stmt), OdbcMatchers::IsError());
  const std::string state = get_sqlstate(stmt);
  CHECK((state == "07006" || state == "22018"));
}

// ============================================================================
// SQL_C_WCHAR -> SQL_TYPE_TIME
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR time string to SQL_TYPE_TIME and read back",
                 "[c_char][conversion][sql_time]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_WCHAR "08:15:00" is bound as the TIME target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'0', '8', ':', '1', '5', ':', '0', '0', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the time is read back correctly, both as text and as a typed TIME struct
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "08:15:00");
  auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
  const SQL_TIME_STRUCT ts = get_data<SQL_C_TYPE_TIME>(struct_stmt, 1);
  CHECK(ts.hour == 8);
  CHECK(ts.minute == 15);
  CHECK(ts.second == 0);
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR with NULL indicator to SQL_TYPE_TIME",
                 "[c_char][conversion][sql_time]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIME, SQL_TYPE_TIME);
  CAPTURE(sql_type);

  // Given a TIME column
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  // When SQL_C_CHAR is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1).has_value());
}
