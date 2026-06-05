// ODBC E2E: SQL_C_INTERVAL_* C types bound via SQLBindParameter to the
// SQL_INTERVAL_* parameter types (interval C -> interval SQL). This suite is
// exhaustive over the two interval families.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// "Converting Interval C Data"), an interval C type may be bound to ANY
// interval SQL target of the SAME family:
//   * year-month family C sources  (SQL_C_INTERVAL_YEAR / _MONTH /
//     _YEAR_TO_MONTH) -> year-month SQL targets (SQL_INTERVAL_YEAR / _MONTH /
//     _YEAR_TO_MONTH); and
//   * day-time family C sources (SQL_C_INTERVAL_DAY / _HOUR / _MINUTE /
//     _SECOND / _DAY_TO_HOUR / _DAY_TO_MINUTE / _DAY_TO_SECOND /
//     _HOUR_TO_MINUTE / _HOUR_TO_SECOND / _MINUTE_TO_SECOND) -> day-time SQL
//     targets (the ten matching SQL_INTERVAL_* subtypes).
// Cross-family binds (a year-month source to a day-time target or vice versa)
// are rejected with SQLSTATE 07006.
//
// The conversion is keyed by the *C source* subtype, not the bound SQL target
// subtype: the driver reads the fields the C subtype carries and renders the
// canonical interval literal for that subtype (e.g. SQL_C_INTERVAL_DAY_TO_SECOND
// renders "1 02:03:04.000000"). The bound SQL_INTERVAL_* parameter type only
// selects the interval family (year-month vs day-time) used for the
// compatibility check; within a family the rendered text is identical across
// every target subtype. Each positive test therefore pins the rendered literal
// for one C subtype and asserts it is unchanged across ALL same-family SQL
// targets. The parameter is inserted into a VARCHAR column so the rendered
// literal is stored verbatim and can be asserted exactly; the driver-side
// formatting is additionally pinned by the Rust unit tests in interval.rs.
//
// The reference driver does not implement binding any SQL_INTERVAL_* parameter
// type, so the positive and 07006-rejection cases are gated behind
// SKIP_OLD_DRIVER("BD#71", ...).

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>
#include <vector>

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

constexpr const char* kBdInterval = "BD#71";
constexpr const char* kBdMessage =
    "Reference driver does not implement binding SQL_C_INTERVAL_* C types to SQL_INTERVAL_* parameters";

// All SQL_INTERVAL_* targets of each family. Within a family every target is a
// legal destination for every same-family C interval source.
const std::vector<SQLSMALLINT> kYearMonthTargets = {
    SQL_INTERVAL_YEAR,
    SQL_INTERVAL_MONTH,
    SQL_INTERVAL_YEAR_TO_MONTH,
};
const std::vector<SQLSMALLINT> kDayTimeTargets = {
    SQL_INTERVAL_DAY,
    SQL_INTERVAL_HOUR,
    SQL_INTERVAL_MINUTE,
    SQL_INTERVAL_SECOND,
    SQL_INTERVAL_DAY_TO_HOUR,
    SQL_INTERVAL_DAY_TO_MINUTE,
    SQL_INTERVAL_DAY_TO_SECOND,
    SQL_INTERVAL_HOUR_TO_MINUTE,
    SQL_INTERVAL_HOUR_TO_SECOND,
    SQL_INTERVAL_MINUTE_TO_SECOND,
};

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

// Binds `val` as (c_type, sql_type), inserts it into a fresh VARCHAR column, and
// returns the round-tripped text. Reuses the fixture connection so a whole
// family grid runs without reconnecting per target.
std::string insert_interval(Connection& conn, SQLSMALLINT c_type, SQLSMALLINT sql_type, SQL_INTERVAL_STRUCT& val) {
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, sql_type, 0, 0, &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  return get_data<SQL_C_CHAR>(fetch_stmt, 1);
}

// Asserts that binding `c_type`/`val` against EVERY target in `targets` (the
// full same-family SQL_INTERVAL_* set) round-trips to the same `expected`
// literal, proving the rendered text is keyed by the C subtype and independent
// of the bound SQL target subtype.
void check_same_family_grid(Connection& conn, SQLSMALLINT c_type, SQL_INTERVAL_STRUCT val, const std::string& expected,
                            const std::vector<SQLSMALLINT>& targets) {
  for (SQLSMALLINT sql_type : targets) {
    CAPTURE(sql_type);
    CHECK(insert_interval(conn, c_type, sql_type, val) == expected);
  }
}

}  // namespace

// ============================================================================
// Year-month family: SQL_C_INTERVAL_YEAR / _MONTH / _YEAR_TO_MONTH bound to
// every year-month SQL_INTERVAL_* target.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR to every year-month SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_YEAR source carrying 5 years
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 5, 0);
  // When it is bound to each year-month SQL_INTERVAL target and inserted
  // Then the year leading field renders as "5" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_YEAR, val, "5", kYearMonthTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MONTH to every year-month SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_MONTH source carrying 11 months
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 0, 11);
  // When it is bound to each year-month SQL_INTERVAL target and inserted
  // Then the month leading field renders as "11" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_MONTH, val, "11", kYearMonthTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR_TO_MONTH to every year-month SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_YEAR_TO_MONTH source carrying 3 years 6 months
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 3, 6);
  // When it is bound to each year-month SQL_INTERVAL target and inserted
  // Then the compound year-month literal renders as "3-06" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_YEAR_TO_MONTH, val, "3-06", kYearMonthTargets);
}

// ============================================================================
// Day-time family: each of the ten SQL_C_INTERVAL_* day-time sources bound to
// every day-time SQL_INTERVAL_* target.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_DAY source carrying 7 days
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 7, 0, 0, 0, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the day leading field renders as "7" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_DAY, val, "7", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_HOUR source carrying 9 hours
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 9, 0, 0, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the hour leading field renders as "9" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_HOUR, val, "9", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_MINUTE source carrying 30 minutes
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 30, 0, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the minute leading field renders as "30" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_MINUTE, val, "30", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_SECOND to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_SECOND source carrying 45 seconds
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 45, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the second leading field renders zero-padded to six fractional digits as "45.000000"
  check_same_family_grid(conn, SQL_C_INTERVAL_SECOND, val, "45.000000", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_HOUR to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_DAY_TO_HOUR source carrying 1 day 2 hours
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 1, 2, 0, 0, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the compound day-hour literal renders as "1 02" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_DAY_TO_HOUR, val, "1 02", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_MINUTE to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_DAY_TO_MINUTE source carrying 1 day 2 hours 3 minutes
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 1, 2, 3, 0, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the compound day-hour-minute literal renders as "1 02:03" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_DAY_TO_MINUTE, val, "1 02:03", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY_TO_SECOND to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_DAY_TO_SECOND source carrying 1 day 2 hours 3 minutes 4 seconds
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 1, 2, 3, 4, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the full compound literal renders as "1 02:03:04.000000" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_DAY_TO_SECOND, val, "1 02:03:04.000000", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR_TO_MINUTE to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_HOUR_TO_MINUTE source carrying 10 hours 30 minutes
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 10, 30, 0, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the compound hour-minute literal renders as "10:30" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_HOUR_TO_MINUTE, val, "10:30", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR_TO_SECOND to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_HOUR_TO_SECOND source carrying 10 hours 30 minutes 15 seconds
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 10, 30, 15, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the compound hour-minute-second literal renders as "10:30:15.000000" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_HOUR_TO_SECOND, val, "10:30:15.000000", kDayTimeTargets);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE_TO_SECOND to every day-time SQL_INTERVAL target",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_MINUTE_TO_SECOND source carrying 30 minutes 15 seconds
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 30, 15, 0);
  // When it is bound to each day-time SQL_INTERVAL target and inserted
  // Then the compound minute-second literal renders as "30:15.000000" for every target subtype
  check_same_family_grid(conn, SQL_C_INTERVAL_MINUTE_TO_SECOND, val, "30:15.000000", kDayTimeTargets);
}

// ============================================================================
// Negative sign — the leading sign of the source struct is applied to the
// rendered literal, for single-field and compound sources in both families.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_YEAR to SQL_INTERVAL_YEAR",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_YEAR source carrying -7 years
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_TRUE, 7, 0);
  // When it is bound to SQL_INTERVAL_YEAR and inserted
  // Then the leading sign is applied to the interval literal "-7"
  CHECK(insert_interval(conn, SQL_C_INTERVAL_YEAR, SQL_INTERVAL_YEAR, val) == "-7");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_YEAR_TO_MONTH to SQL_INTERVAL_YEAR_TO_MONTH",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_YEAR_TO_MONTH source carrying -2 years 3 months
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_TRUE, 2, 3);
  // When it is bound to SQL_INTERVAL_YEAR_TO_MONTH and inserted
  // Then the leading sign is applied to the compound literal "-2-03"
  CHECK(insert_interval(conn, SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_INTERVAL_YEAR_TO_MONTH, val) == "-2-03");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_SECOND to SQL_INTERVAL_SECOND",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_SECOND source carrying -3 seconds
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 0, 0, 0, 3, 0);
  // When it is bound to SQL_INTERVAL_SECOND and inserted
  // Then the leading sign is applied to the interval literal "-3.000000"
  CHECK(insert_interval(conn, SQL_C_INTERVAL_SECOND, SQL_INTERVAL_SECOND, val) == "-3.000000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative SQL_C_INTERVAL_DAY_TO_SECOND to SQL_INTERVAL_DAY_TO_SECOND",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_DAY_TO_SECOND source carrying -(1 day 2 hours 3 minutes 4 seconds)
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_TRUE, 1, 2, 3, 4, 0);
  // When it is bound to SQL_INTERVAL_DAY_TO_SECOND and inserted
  // Then the leading sign is applied to the compound literal "-1 02:03:04.000000"
  CHECK(insert_interval(conn, SQL_C_INTERVAL_DAY_TO_SECOND, SQL_INTERVAL_DAY_TO_SECOND, val) == "-1 02:03:04.000000");
}

// ============================================================================
// Fractional seconds — the microsecond `fraction` field is rendered zero-padded
// to six digits, for both the leading-field and compound seconds positions.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind fractional SQL_C_INTERVAL_SECOND to SQL_INTERVAL_SECOND",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a SQL_C_INTERVAL_SECOND source carrying 12.5 seconds (500000 microseconds)
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, 12, 500'000);
  // When it is bound to SQL_INTERVAL_SECOND and inserted
  // Then the fractional component renders zero-padded to six digits as "12.500000"
  CHECK(insert_interval(conn, SQL_C_INTERVAL_SECOND, SQL_INTERVAL_SECOND, val) == "12.500000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind fractional SQL_C_INTERVAL_DAY_TO_SECOND to SQL_INTERVAL_DAY_TO_SECOND",
                 "[c_interval][conversion][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  // Given a compound SQL_C_INTERVAL_DAY_TO_SECOND source carrying 1 day 2 hours 3 minutes 4.5 seconds
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 1, 2, 3, 4, 500'000);
  // When it is bound to SQL_INTERVAL_DAY_TO_SECOND and inserted
  // Then the compound seconds sub-field renders zero-padded to six digits as "1 02:03:04.500000"
  CHECK(insert_interval(conn, SQL_C_INTERVAL_DAY_TO_SECOND, SQL_INTERVAL_DAY_TO_SECOND, val) == "1 02:03:04.500000");
}

// ============================================================================
// Cross-family binds are rejected with 07006 — a year-month source cannot be
// bound to a day-time target and vice versa, for single-field AND compound
// sources/targets.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject cross-family SQL_C_INTERVAL to SQL_INTERVAL with 07006",
                 "[c_interval][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(kBdInterval, kBdMessage);
  auto [c_type, sql_type] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLSMALLINT>({
      // year-month source -> day-time target
      {SQL_C_INTERVAL_YEAR, SQL_INTERVAL_DAY},
      {SQL_C_INTERVAL_MONTH, SQL_INTERVAL_SECOND},
      {SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_INTERVAL_DAY},
      {SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_INTERVAL_DAY_TO_SECOND},
      {SQL_C_INTERVAL_MONTH, SQL_INTERVAL_HOUR_TO_MINUTE},
      // day-time source -> year-month target
      {SQL_C_INTERVAL_DAY, SQL_INTERVAL_YEAR},
      {SQL_C_INTERVAL_SECOND, SQL_INTERVAL_MONTH},
      {SQL_C_INTERVAL_DAY_TO_SECOND, SQL_INTERVAL_YEAR},
      {SQL_C_INTERVAL_DAY_TO_SECOND, SQL_INTERVAL_YEAR_TO_MONTH},
      {SQL_C_INTERVAL_MINUTE_TO_SECOND, SQL_INTERVAL_YEAR_TO_MONTH},
  }));
  CAPTURE(c_type, sql_type);

  // Given a prepared statement targeting a VARCHAR column and an interval struct
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col VARCHAR(200))");
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
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE t (col VARCHAR(200))");

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
