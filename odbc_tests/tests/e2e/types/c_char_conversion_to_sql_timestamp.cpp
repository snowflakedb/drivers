// ODBC E2E: SQL_C_CHAR / SQL_C_WCHAR bound via SQLBindParameter to SQL_TYPE_TIMESTAMP.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types"),
// SQL_C_CHAR and SQL_C_WCHAR are legal source types for SQL_TYPE_TIMESTAMP. The
// driver parses the character literal as "YYYY-MM-DD HH:MM:SS" (optionally with
// a fractional-seconds component) and forwards a typed TIMESTAMP value;
// well-formed literals round-trip through a TIMESTAMP_NTZ column unchanged.
//
// The new driver stores the literal verbatim in the timezone-naive TIMESTAMP_NTZ
// column. The legacy driver instead shifts the wall-clock value by the session
// timezone offset before storing (BD#74): with the session pinned to Asia/Dubai
// (UTC+4), "14:30:45" is stored as "18:30:45". The shift depends only on the session
// timezone, not the host, so the stored value is deterministic and both drivers'
// read-backs are asserted exactly under OLD_DRIVER_ONLY / NEW_DRIVER_ONLY.
// Malformed literals are rejected by both drivers (the new driver surfaces SQLSTATE
// 07006, the legacy driver 22018), so the negative case asserts one of those two.
//
// Per ODBC Appendix G ("Driver Guidelines for Backward Compatibility"), the
// ODBC 3.x code SQL_TYPE_TIMESTAMP (93) and its ODBC 2.x predecessor
// SQL_TIMESTAMP (11) must be accepted as identical at the SQLBindParameter
// boundary, so every case is parametrized over both spellings with Catch2 GENERATE.

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
// SQL_C_CHAR -> SQL_TYPE_TIMESTAMP
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR timestamp string to SQL_TYPE_TIMESTAMP and read back",
                 "[c_char][conversion][sql_timestamp]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // Given a TIMESTAMP_NTZ column; session timezone pinned to Asia/Dubai (UTC+4) so the old driver's
  // timezone-shifted stored value is deterministic and distinguishable from the new driver's verbatim value
  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Dubai'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_CHAR "2024-01-15 14:30:45" is bound as the TIMESTAMP target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "2024-01-15 14:30:45";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  OLD_DRIVER_ONLY("BD#74") {
    // Then the old driver stores the value shifted by the session TZ offset (+4h for Asia/Dubai)
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15 18:30:45");
  }
  NEW_DRIVER_ONLY("BD#74") {
    // Then the timestamp is read back correctly, both as text and as a typed TIMESTAMP struct
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15 14:30:45");
    auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
    const SQL_TIMESTAMP_STRUCT ts = get_data<SQL_C_TYPE_TIMESTAMP>(struct_stmt, 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 14);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should bind SQL_C_CHAR timestamp string with fractional seconds to SQL_TYPE_TIMESTAMP",
                 "[c_char][conversion][sql_timestamp]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // Given a TIMESTAMP_NTZ(9) column; session timezone pinned to Asia/Dubai (UTC+4)
  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Dubai'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ(9))");

  // When SQL_C_CHAR "2024-01-15 14:30:45.123456789" is bound as the TIMESTAMP target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "2024-01-15 14:30:45.123456789";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 9, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  OLD_DRIVER_ONLY("BD#74") {
    // Then the old driver stores the value shifted by the session TZ offset (+4h for Asia/Dubai)
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15 18:30:45.123456789");
  }
  NEW_DRIVER_ONLY("BD#74") {
    // Then the timestamp with fractional seconds is read back correctly, both as text and
    // as a typed TIMESTAMP struct (fraction is expressed in nanoseconds)
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15 14:30:45.123456789");
    auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
    const SQL_TIMESTAMP_STRUCT ts = get_data<SQL_C_TYPE_TIMESTAMP>(struct_stmt, 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 14);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 123456789);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR malformed timestamp string for SQL_TYPE_TIMESTAMP",
                 "[c_char][conversion][sql_timestamp]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // Given a TIMESTAMP_NTZ column
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When a non-timestamp SQL_C_CHAR literal is bound as the TIMESTAMP target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "not-a-timestamp";
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
// SQL_C_WCHAR -> SQL_TYPE_TIMESTAMP
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR timestamp string to SQL_TYPE_TIMESTAMP and read back",
                 "[c_char][conversion][sql_timestamp]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // Given a TIMESTAMP_NTZ column; session timezone pinned to Asia/Dubai (UTC+4) so the old driver's
  // timezone-shifted stored value is deterministic and distinguishable from the new driver's verbatim value
  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Dubai'");
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

  // When SQL_C_WCHAR "2024-01-15 14:30:45" is bound as the TIMESTAMP target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'2', '0', '2', '4', '-', '0', '1', '-', '1', '5', ' ', '1', '4', ':', '3', '0', ':', '4', '5', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  OLD_DRIVER_ONLY("BD#74") {
    // Then the old driver stores the value shifted by the session TZ offset (+4h for Asia/Dubai)
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15 18:30:45");
  }
  NEW_DRIVER_ONLY("BD#74") {
    // Then the timestamp is read back correctly, both as text and as a typed TIMESTAMP struct
    auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
    CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15 14:30:45");
    auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
    const SQL_TIMESTAMP_STRUCT ts = get_data<SQL_C_TYPE_TIMESTAMP>(struct_stmt, 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 14);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR with NULL indicator to SQL_TYPE_TIMESTAMP",
                 "[c_char][conversion][sql_timestamp]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_TIMESTAMP, SQL_TYPE_TIMESTAMP);
  CAPTURE(sql_type);

  // Given a TIMESTAMP_NTZ column
  conn.execute("CREATE TEMPORARY TABLE t (col TIMESTAMP_NTZ)");

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
