// ODBC E2E: SQL_C_CHAR / SQL_C_WCHAR bound via SQLBindParameter to SQL_TYPE_DATE.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types"),
// SQL_C_CHAR and SQL_C_WCHAR are legal source types for SQL_TYPE_DATE. The
// driver parses the character literal as "YYYY-MM-DD" and forwards a typed DATE
// value; well-formed literals round-trip through a DATE column unchanged.
//
// Both drivers accept well-formed literals, so the positive cases run on the
// reference driver too. Malformed literals are rejected by both drivers (the
// new driver surfaces SQLSTATE 07006, the legacy driver 22018), so the negative
// case asserts the error surfaces one of those two conversion SQLSTATEs.
//
// Per ODBC Appendix G ("Driver Guidelines for Backward Compatibility"), the
// ODBC 3.x code SQL_TYPE_DATE (91) and its ODBC 2.x predecessor SQL_DATE (9)
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
// SQL_C_CHAR -> SQL_TYPE_DATE
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR date string to SQL_TYPE_DATE and read back",
                 "[c_char][conversion][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_CHAR "2024-01-15" is bound as the DATE target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "2024-01-15";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the date is read back correctly, both as text and as a typed DATE struct
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15");
  auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
  const SQL_DATE_STRUCT d = get_data<SQL_C_TYPE_DATE>(struct_stmt, 1);
  CHECK(d.year == 2024);
  CHECK(d.month == 1);
  CHECK(d.day == 15);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR end-of-year date string to SQL_TYPE_DATE",
                 "[c_char][conversion][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_CHAR "2024-12-31" is bound as the DATE target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "2024-12-31";
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the date is read back correctly, both as text and as a typed DATE struct
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-12-31");
  auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
  const SQL_DATE_STRUCT d = get_data<SQL_C_TYPE_DATE>(struct_stmt, 1);
  CHECK(d.year == 2024);
  CHECK(d.month == 12);
  CHECK(d.day == 31);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_CHAR malformed date string for SQL_TYPE_DATE",
                 "[c_char][conversion][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When a non-date SQL_C_CHAR literal is bound as the DATE target
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  char val[] = "not-a-date";
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
// SQL_C_WCHAR -> SQL_TYPE_DATE
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_WCHAR date string to SQL_TYPE_DATE and read back",
                 "[c_char][conversion][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  // When SQL_C_WCHAR "2024-01-15" is bound as the DATE target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLWCHAR val[] = {'2', '0', '2', '4', '-', '0', '1', '-', '1', '5', 0};
  SQLLEN ind = SQL_NTS;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_WCHAR, sql_type, 0, 0, val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the date is read back correctly, both as text and as a typed DATE struct
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "2024-01-15");
  auto struct_stmt = conn.execute_fetch("SELECT col FROM t");
  const SQL_DATE_STRUCT d = get_data<SQL_C_TYPE_DATE>(struct_stmt, 1);
  CHECK(d.year == 2024);
  CHECK(d.month == 1);
  CHECK(d.day == 15);
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_CHAR with NULL indicator to SQL_TYPE_DATE",
                 "[c_char][conversion][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given a DATE column
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

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
