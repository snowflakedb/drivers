// ODBC E2E: SQL_C_INTERVAL_DAY / HOUR / MINUTE / SECOND bound via
// SQLBindParameter to the exact-numeric SQL targets - both the native
// integer types (SQL_TINYINT / SQL_SMALLINT / SQL_INTEGER / SQL_BIGINT)
// and the variable-precision types (SQL_DECIMAL / SQL_NUMERIC) routed
// through `DecimalParamConverter` in `param_binding.rs`.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// "Converting Interval C Data"), a single-field interval bound to an
// exact-numeric target carries the magnitude of its single datetime
// field with the `interval_sign` applied. Fractional seconds carried
// by SQL_C_INTERVAL_SECOND are truncated when the target is an exact
// integer type.

#include <sql.h>
#include <sqlext.h>

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

// SQL_DECIMAL / SQL_NUMERIC bindings carry meaningful column-size /
// decimal-digits; the integer SQL targets ignore them and the existing
// call sites pass 0/0, so default the parameters that way.
void bind_interval_and_execute(StatementHandleWrapper& stmt, SQLSMALLINT c_type, SQLSMALLINT sql_type,
                               SQL_INTERVAL_STRUCT& val, SQLLEN& ind, SQLULEN column_size = 0,
                               SQLSMALLINT decimal_digits = 0) {
  SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, sql_type, column_size, decimal_digits,
                                   &val, sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
}

}  // namespace

// ============================================================================
// SQL_C_INTERVAL_DAY -> exact-numeric SQL targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_DAY to exact-numeric SQL targets and read back",
                 "[c_interval][conversion][sql_fixed]") {
  auto [sql_type, column_size, days] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLULEN, SQLUINTEGER>({
      {SQL_TINYINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(15)},
      {SQL_SMALLINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(365)},
      {SQL_INTEGER, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(100000)},
      {SQL_BIGINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(9876543)},
      {SQL_DECIMAL, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(9876543)},
      {SQL_NUMERIC, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(9876543)},
  }));
  CAPTURE(sql_type, column_size, days);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When SQL_C_INTERVAL_DAY carrying `days` is bound to the SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, days, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_DAY, sql_type, val, ind, column_size, 0);

  // Then the day magnitude is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == static_cast<SQLBIGINT>(days));
}

// ============================================================================
// SQL_C_INTERVAL_HOUR -> exact-numeric SQL targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_HOUR to exact-numeric SQL targets and read back",
                 "[c_interval][conversion][sql_fixed]") {
  auto [sql_type, column_size, hours] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLULEN, SQLUINTEGER>({
      {SQL_TINYINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(8)},
      {SQL_SMALLINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(240)},
      {SQL_INTEGER, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(100000)},
      {SQL_BIGINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(1234567)},
      {SQL_DECIMAL, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(1234567)},
      {SQL_NUMERIC, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(1234567)},
  }));
  CAPTURE(sql_type, column_size, hours);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When SQL_C_INTERVAL_HOUR carrying `hours` is bound to the SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, hours, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_HOUR, sql_type, val, ind, column_size, 0);

  // Then the hour magnitude is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == static_cast<SQLBIGINT>(hours));
}

// ============================================================================
// SQL_C_INTERVAL_MINUTE -> exact-numeric SQL targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MINUTE to exact-numeric SQL targets and read back",
                 "[c_interval][conversion][sql_fixed]") {
  auto [sql_type, column_size, minutes] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLULEN, SQLUINTEGER>({
      {SQL_TINYINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(30)},
      {SQL_SMALLINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(1440)},
      {SQL_INTEGER, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(525600)},
      {SQL_BIGINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(78901234)},
      {SQL_DECIMAL, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(78901234)},
      {SQL_NUMERIC, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(78901234)},
  }));
  CAPTURE(sql_type, column_size, minutes);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When SQL_C_INTERVAL_MINUTE carrying `minutes` is bound to the SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, minutes, 0, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MINUTE, sql_type, val, ind, column_size, 0);

  // Then the minute magnitude is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == static_cast<SQLBIGINT>(minutes));
}

// ============================================================================
// SQL_C_INTERVAL_SECOND -> exact-numeric SQL targets
//
// The seconds fraction is carried in the struct but must be dropped
// when the target is an exact integer per ODBC Appendix D ("Converting
// Data from C to SQL Data Types: Truncation of Interval Values"). Two
// of the rows below carry a non-zero `fraction` to pin that contract
// for both the small and wide targets.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should bind SQL_C_INTERVAL_SECOND to exact-numeric SQL targets and truncate fraction",
                 "[c_interval][conversion][sql_fixed]") {
  auto [sql_type, column_size, seconds,
        fraction] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLULEN, SQLUINTEGER, SQLUINTEGER>({
      {SQL_TINYINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(45), static_cast<SQLUINTEGER>(999'999'999)},
      {SQL_SMALLINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(3600), static_cast<SQLUINTEGER>(0)},
      {SQL_INTEGER, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(86400), static_cast<SQLUINTEGER>(500'000'000)},
      {SQL_BIGINT, static_cast<SQLULEN>(0), static_cast<SQLUINTEGER>(1234567890), static_cast<SQLUINTEGER>(0)},
      {SQL_DECIMAL, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(1234567890),
       static_cast<SQLUINTEGER>(500'000'000)},
      {SQL_NUMERIC, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(1234567890),
       static_cast<SQLUINTEGER>(500'000'000)},
      {SQL_NUMERIC, static_cast<SQLULEN>(10), static_cast<SQLUINTEGER>(1234567890), static_cast<SQLUINTEGER>(0)},
  }));
  CAPTURE(sql_type, column_size, seconds, fraction);

  const bool variable_precision = (sql_type == SQL_DECIMAL || sql_type == SQL_NUMERIC);
  const bool has_fraction = fraction != 0;

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When SQL_C_INTERVAL_SECOND carrying `seconds` and `fraction` is bound to the SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 0, 0, 0, seconds, fraction);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_SECOND, sql_type, column_size, 0, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // BD#60: the old driver rejects a non-zero seconds fraction bound to
  // SQL_DECIMAL / SQL_NUMERIC with 22015 (NativeError 40530). Integer SQL
  // targets, and variable-precision targets with a zero fraction, truncate
  // and store on both drivers.
  if (variable_precision && has_fraction) {
    OLD_DRIVER_ONLY("BD#60") {
      CHECK(ret == SQL_ERROR);
      auto records = get_diag_rec(stmt);
      REQUIRE_FALSE(records.empty());
      CHECK(records[0].sqlState == "22015");
      CHECK(records[0].nativeError == 40530);
      return;
    }
  }

  REQUIRE_ODBC(ret, stmt);

  // Then only the integral seconds magnitude is stored (fraction is dropped)
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == static_cast<SQLBIGINT>(seconds));
}

// ============================================================================
// Negative sign — every single-field source maps its `interval_sign` onto the
// stored value, with the magnitude living in the corresponding sub-field of
// `intval.day_second`. Parameterized over all four sources to pin the
// contract once.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative day-time interval to SQL_INTEGER and read back",
                 "[c_interval][conversion][sql_fixed]") {
  auto [c_type, magnitude] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLUINTEGER>({
      {SQL_C_INTERVAL_DAY, static_cast<SQLUINTEGER>(15)},
      {SQL_C_INTERVAL_HOUR, static_cast<SQLUINTEGER>(8)},
      {SQL_C_INTERVAL_MINUTE, static_cast<SQLUINTEGER>(30)},
      {SQL_C_INTERVAL_SECOND, static_cast<SQLUINTEGER>(45)},
  }));
  CAPTURE(c_type, magnitude);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When the day-time interval carrying -magnitude is bound to SQL_INTEGER and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = {};
  val.interval_sign = SQL_TRUE;
  switch (c_type) {
    case SQL_C_INTERVAL_DAY:
      val.intval.day_second.day = magnitude;
      break;
    case SQL_C_INTERVAL_HOUR:
      val.intval.day_second.hour = magnitude;
      break;
    case SQL_C_INTERVAL_MINUTE:
      val.intval.day_second.minute = magnitude;
      break;
    case SQL_C_INTERVAL_SECOND:
      val.intval.day_second.second = magnitude;
      break;
  }
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, c_type, SQL_INTEGER, val, ind);

  // Then the leading sign is applied to the magnitude
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -static_cast<SQLBIGINT>(magnitude));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_INTERVAL_DAY overflow into NUMBER(3,0)",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a narrow NUMBER(3,0) column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER(3,0))");

  // When SQL_C_INTERVAL_DAY carrying 99999 (5 digits) is bound to SQL_DECIMAL and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 99999, 0, 0, 0, 0);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_DAY, SQL_DECIMAL, 10, 0, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the server rejects the value with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject compound day-time interval bound to SQL_INTEGER with 07006",
                 "[c_interval][incompatible][sql_fixed]") {
  const SQLSMALLINT c_type =
      GENERATE(SQL_C_INTERVAL_DAY_TO_HOUR, SQL_C_INTERVAL_DAY_TO_MINUTE, SQL_C_INTERVAL_DAY_TO_SECOND,
               SQL_C_INTERVAL_HOUR_TO_MINUTE, SQL_C_INTERVAL_HOUR_TO_SECOND, SQL_C_INTERVAL_MINUTE_TO_SECOND);
  CAPTURE(c_type);

  // Given a prepared statement targeting a NUMBER column and a compound day-time interval struct
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ds_interval(SQL_FALSE, 1, 2, 3, 4, 0);
  SQLLEN ind = sizeof(val);

  // When the compound day-time interval is bound to SQL_INTEGER and executed
  // Then the driver rejects the incompatible conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, c_type, SQL_INTEGER, &val, sizeof(val), &ind);
}

// ============================================================================
// NULL indicator
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind day-time interval with NULL indicator to SQL_INTEGER",
                 "[c_interval][conversion][sql_fixed]") {
  const SQLSMALLINT c_type =
      GENERATE(SQL_C_INTERVAL_DAY, SQL_C_INTERVAL_HOUR, SQL_C_INTERVAL_MINUTE, SQL_C_INTERVAL_SECOND);
  CAPTURE(c_type);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When the day-time interval is bound with SQL_NULL_DATA and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, c_type, SQL_INTEGER, 0, 0, nullptr, 0, &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_SBIGINT>(fetch_stmt, 1).has_value());
}
