// ODBC E2E: single-field SQL_C_INTERVAL_* C types bound via SQLBindParameter
// to single-field SQL_INTERVAL_* parameter types (interval C -> interval SQL).
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// "Converting Interval C Data"), an interval C type may be bound to an
// interval SQL target of the SAME family — a year-month interval C source
// (SQL_C_INTERVAL_YEAR / _MONTH) to a year-month SQL target
// (SQL_INTERVAL_YEAR / _MONTH), and a day-time interval C source
// (SQL_C_INTERVAL_DAY / _HOUR / _MINUTE / _SECOND) to a day-time SQL target
// (SQL_INTERVAL_DAY / _HOUR / _MINUTE / _SECOND). Cross-family binds (a
// year-month source to a day-time target or vice versa) are rejected with
// SQLSTATE 07006.
//
// The driver forwards the source interval's leading datetime field as the
// rendered literal regardless of the bound SQL_INTERVAL_* subtype (e.g. a
// SQL_C_INTERVAL_YEAR carrying 5 years renders "5" whether the target is
// SQL_INTERVAL_YEAR or SQL_INTERVAL_MONTH). The conversion under test is keyed
// by the bound SQL_INTERVAL_* parameter type, not the column type, so the
// parameter is bound as SQL_INTERVAL_* and inserted into a VARCHAR column;
// because the column is VARCHAR the rendered literal is stored verbatim, so
// these tests assert the exact round-tripped text. The driver-side formatting
// itself is additionally pinned by the Rust unit tests in interval.rs.
//
// The reference driver does not implement binding any SQL_INTERVAL_* parameter
// type, so the positive and 07006-rejection cases are gated behind
// SKIP_OLD_DRIVER("BD#62", ...).

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

constexpr const char* kBdInterval = "BD#62";
constexpr const char* kBdMessage =
    "Reference driver does not implement binding SQL_C_INTERVAL_* C types to SQL_INTERVAL_* parameters";

SQL_INTERVAL_STRUCT ym_interval(SQLSMALLINT sign, SQLUINTEGER year, SQLUINTEGER month) {
  SQL_INTERVAL_STRUCT iv = {};
  iv.interval_sign = sign;
  iv.intval.year_month.year = year;
  iv.intval.year_month.month = month;
  return iv;
}

SQL_INTERVAL_STRUCT ds_interval(SQLSMALLINT sign, SQLUINTEGER day, SQLUINTEGER hour, SQLUINTEGER minute,
                                SQLUINTEGER second, SQLUINTEGER fraction) {
  SQL_INTERVAL_STRUCT iv = {};
  iv.interval_sign = sign;
  iv.intval.day_second.day = day;
  iv.intval.day_second.hour = hour;
  iv.intval.day_second.minute = minute;
  iv.intval.day_second.second = second;
  iv.intval.day_second.fraction = fraction;
  return iv;
}

void bind_interval_and_execute(StatementHandleWrapper& stmt, SQLSMALLINT c_type, SQLSMALLINT sql_type,
                               SQL_INTERVAL_STRUCT& val, SQLLEN& ind) {
  SQLRETURN ret =
      SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, sql_type, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// Year-month family: SQL_C_INTERVAL_YEAR / _MONTH -> SQL_INTERVAL_YEAR / _MONTH
//
// The rendered literal is the source's leading field magnitude, independent of
// which year-month SQL subtype is bound.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR to year-month SQL_INTERVAL targets",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  const SQLSMALLINT target = GENERATE(SQL_INTERVAL_YEAR, SQL_INTERVAL_MONTH);
  CAPTURE(target);

  // Given a VARCHAR column (sufficient to exercise the bound interval parameter type)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_YEAR carrying 5 years is bound to the year-month SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 5, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR, target, val, ind);

  // Then the year leading field is forwarded as the interval literal "5"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MONTH to year-month SQL_INTERVAL targets",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  const SQLSMALLINT target = GENERATE(SQL_INTERVAL_YEAR, SQL_INTERVAL_MONTH);
  CAPTURE(target);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_MONTH carrying 11 months is bound to the year-month SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 0, 11);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MONTH, target, val, ind);

  // Then the month leading field is forwarded as the interval literal "11"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "11");
}

// ============================================================================
// Day-time family: SQL_C_INTERVAL_DAY / _HOUR / _MINUTE / _SECOND ->
// SQL_INTERVAL_DAY / _HOUR / _MINUTE / _SECOND
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY to day-time SQL_INTERVAL targets",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  const SQLSMALLINT target = GENERATE(SQL_INTERVAL_DAY, SQL_INTERVAL_HOUR, SQL_INTERVAL_MINUTE, SQL_INTERVAL_SECOND);
  CAPTURE(target);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_DAY carrying 7 days is bound to the day-time SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 7, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY, target, val, ind);

  // Then the day leading field is forwarded as the interval literal "7"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "7");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR to day-time SQL_INTERVAL targets",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  const SQLSMALLINT target = GENERATE(SQL_INTERVAL_DAY, SQL_INTERVAL_HOUR, SQL_INTERVAL_MINUTE, SQL_INTERVAL_SECOND);
  CAPTURE(target);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_HOUR carrying 9 hours is bound to the day-time SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 9, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR, target, val, ind);

  // Then the hour leading field is forwarded as the interval literal "9"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "9");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE to day-time SQL_INTERVAL targets",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  const SQLSMALLINT target = GENERATE(SQL_INTERVAL_DAY, SQL_INTERVAL_HOUR, SQL_INTERVAL_MINUTE, SQL_INTERVAL_SECOND);
  CAPTURE(target);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_MINUTE carrying 30 minutes is bound to the day-time SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 30, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MINUTE, target, val, ind);

  // Then the minute leading field is forwarded as the interval literal "30"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "30");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND to day-time SQL_INTERVAL targets",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  const SQLSMALLINT target = GENERATE(SQL_INTERVAL_DAY, SQL_INTERVAL_HOUR, SQL_INTERVAL_MINUTE, SQL_INTERVAL_SECOND);
  CAPTURE(target);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_SECOND carrying 45 seconds is bound to the day-time SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 45, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, target, val, ind);

  // Then the second leading field is forwarded with six fractional digits as "45.000000"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.000000");
}

// ============================================================================
// Negative sign — the leading sign of the source struct is applied to the
// rendered literal. Pinned for one year-month and one day-time source.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_YEAR to SQL_INTERVAL_YEAR",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_YEAR carrying -7 years is bound to SQL_INTERVAL_YEAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_TRUE, 7, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR, SQL_INTERVAL_YEAR, val, ind);

  // Then the leading sign is applied to the interval literal "-7"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-7");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_SECOND to SQL_INTERVAL_SECOND",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_SECOND carrying -3 seconds is bound to SQL_INTERVAL_SECOND and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 0, 0, 3, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, SQL_INTERVAL_SECOND, val, ind);

  // Then the leading sign is applied to the interval literal "-3.000000"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "-3.000000");
}

// ============================================================================
// Fractional seconds — SQL_C_INTERVAL_SECOND carries a microsecond fraction
// that is rendered zero-padded to six digits.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind fractional SQL_C_INTERVAL_SECOND to SQL_INTERVAL_SECOND",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_SECOND carrying 12.5 seconds is bound to SQL_INTERVAL_SECOND and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 12, 500'000);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_SECOND, SQL_INTERVAL_SECOND, val, ind);

  // Then the fractional component is rendered zero-padded to six digits as "12.500000"
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "12.500000");
}

// ============================================================================
// Cross-family binds are rejected with 07006 — a year-month source cannot be
// bound to a day-time target and vice versa.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject cross-family SQL_C_INTERVAL to SQL_INTERVAL with 07006",
                 "[c_interval][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  auto [c_type, sql_type] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLSMALLINT>({
      {SQL_C_INTERVAL_YEAR, SQL_INTERVAL_DAY},
      {SQL_C_INTERVAL_MONTH, SQL_INTERVAL_SECOND},
      {SQL_C_INTERVAL_DAY, SQL_INTERVAL_YEAR},
      {SQL_C_INTERVAL_SECOND, SQL_INTERVAL_MONTH},
  }));
  CAPTURE(c_type, sql_type);

  // Given a prepared statement targeting a VARCHAR column and an interval struct
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 1, 1, 1, 1, 0);
  SQLLEN ind = sizeof(val);

  // When the cross-family interval source is bound to the mismatched-family SQL_INTERVAL target and executed
  // Then the driver rejects the incompatible conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, c_type, sql_type, &val, sizeof(val), &ind);
}

// ============================================================================
// NULL indicator
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR with NULL indicator to SQL_INTERVAL_YEAR",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a SQL_C_INTERVAL_YEAR is bound with SQL_NULL_DATA to SQL_INTERVAL_YEAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_YEAR, SQL_INTERVAL_YEAR, 0, 0, nullptr, 0,
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1).has_value());
}
