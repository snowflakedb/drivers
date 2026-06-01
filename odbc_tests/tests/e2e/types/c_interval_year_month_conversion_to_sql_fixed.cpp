// ODBC E2E: SQL_C_INTERVAL_YEAR / SQL_C_INTERVAL_MONTH bound via
// SQLBindParameter to the exact-numeric SQL targets — both the native
// integer types (SQL_TINYINT / SQL_SMALLINT / SQL_INTEGER / SQL_BIGINT)
// and the variable-precision types (SQL_DECIMAL / SQL_NUMERIC) routed
// through `DecimalParamConverter` in `param_binding.rs`.
//
// Per ODBC Appendix D ("Converting Data from C to SQL Data Types",
// "Converting Interval C Data"), a single-field interval bound to an
// exact-numeric target carries the magnitude of its single datetime
// field (years for INTERVAL_YEAR, months for INTERVAL_MONTH) with the
// `interval_sign` applied.

#include <sql.h>
#include <sqlext.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_INTERVAL_STRUCT ym_interval(SQLSMALLINT sign, SQLUINTEGER year, SQLUINTEGER month) {
  SQL_INTERVAL_STRUCT iv = {};
  iv.interval_sign = sign;
  iv.intval.year_month.year = year;
  iv.intval.year_month.month = month;
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
// SQL_C_INTERVAL_YEAR -> exact-numeric SQL targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_YEAR to exact-numeric SQL targets and read back",
                 "[c_interval][conversion][sql_fixed]") {
  auto [sql_type, column_size, years] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLULEN, SQLUINTEGER>({
      {SQL_TINYINT, SQLULEN{0}, SQLUINTEGER{5}},
      {SQL_SMALLINT, SQLULEN{0}, SQLUINTEGER{200}},
      {SQL_INTEGER, SQLULEN{0}, SQLUINTEGER{12345}},
      {SQL_BIGINT, SQLULEN{0}, SQLUINTEGER{1000000}},
      {SQL_DECIMAL, SQLULEN{10}, SQLUINTEGER{1000000}},
      {SQL_NUMERIC, SQLULEN{10}, SQLUINTEGER{1000000}},
  }));
  CAPTURE(sql_type, column_size, years);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When SQL_C_INTERVAL_YEAR carrying `years` is bound to the SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, years, 0);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_YEAR, sql_type, val, ind, column_size, 0);

  // Then the year magnitude is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == static_cast<SQLBIGINT>(years));
}

// ============================================================================
// SQL_C_INTERVAL_MONTH -> exact-numeric SQL targets
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_INTERVAL_MONTH to exact-numeric SQL targets and read back",
                 "[c_interval][conversion][sql_fixed]") {
  auto [sql_type, column_size, months] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLULEN, SQLUINTEGER>({
      {SQL_TINYINT, SQLULEN{0}, SQLUINTEGER{11}},
      {SQL_SMALLINT, SQLULEN{0}, SQLUINTEGER{240}},
      {SQL_INTEGER, SQLULEN{0}, SQLUINTEGER{9999}},
      {SQL_BIGINT, SQLULEN{0}, SQLUINTEGER{123456}},
      {SQL_DECIMAL, SQLULEN{10}, SQLUINTEGER{123456}},
      {SQL_NUMERIC, SQLULEN{10}, SQLUINTEGER{123456}},
  }));
  CAPTURE(sql_type, column_size, months);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When SQL_C_INTERVAL_MONTH carrying `months` is bound to the SQL target and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 0, months);
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, SQL_C_INTERVAL_MONTH, sql_type, val, ind, column_size, 0);

  // Then only the month sub-field contributes and the value is stored
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == static_cast<SQLBIGINT>(months));
}

// ============================================================================
// Negative sign — both single-field sources map their `interval_sign` onto
// the stored value, with the magnitude living in the corresponding sub-field
// of `intval.year_month`. Parameterized over both sources to pin the
// contract once.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind negative year-month interval to SQL_INTEGER and read back",
                 "[c_interval][conversion][sql_fixed]") {
  auto [c_type, magnitude] = GENERATE(Catch::Generators::table<SQLSMALLINT, SQLUINTEGER>({
      {SQL_C_INTERVAL_YEAR, SQLUINTEGER{7}},
      {SQL_C_INTERVAL_MONTH, SQLUINTEGER{8}},
  }));
  CAPTURE(c_type, magnitude);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When the year-month interval carrying -magnitude is bound to SQL_INTEGER and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = {};
  val.interval_sign = SQL_TRUE;
  switch (c_type) {
    case SQL_C_INTERVAL_YEAR:
      val.intval.year_month.year = magnitude;
      break;
    case SQL_C_INTERVAL_MONTH:
      val.intval.year_month.month = magnitude;
      break;
  }
  SQLLEN ind = sizeof(val);
  bind_interval_and_execute(stmt, c_type, SQL_INTEGER, val, ind);

  // Then the leading sign is applied to the magnitude
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_SBIGINT>(fetch_stmt, 1) == -static_cast<SQLBIGINT>(magnitude));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_INTERVAL_YEAR overflow into NUMBER(3,0)",
                 "[c_interval][conversion][sql_fixed]") {
  // Given a narrow NUMBER(3,0) column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER(3,0))");

  // When SQL_C_INTERVAL_YEAR carrying 99999 (5 digits) is bound to SQL_DECIMAL and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 99999, 0);
  SQLLEN ind = sizeof(val);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_INTERVAL_YEAR, SQL_DECIMAL, 10, 0, &val,
                         sizeof(val), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the server rejects the value with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22003");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_INTERVAL_YEAR_TO_MONTH bound to SQL_INTEGER with 07006",
                 "[c_interval][incompatible][sql_fixed]") {
  // Given a prepared statement targeting a NUMBER column and a SQL_C_INTERVAL_YEAR_TO_MONTH struct (1y, 6m)
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_INTERVAL_STRUCT val = ym_interval(SQL_FALSE, 1, 6);
  SQLLEN ind = sizeof(val);

  // When SQL_C_INTERVAL_YEAR_TO_MONTH is bound to SQL_INTEGER and executed
  // Then the driver rejects the incompatible conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_INTEGER, &val, sizeof(val), &ind);
}

// ============================================================================
// NULL indicator
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should bind year-month interval with NULL indicator to SQL_INTEGER",
                 "[c_interval][conversion][sql_fixed]") {
  const SQLSMALLINT c_type = GENERATE(SQL_C_INTERVAL_YEAR, SQL_C_INTERVAL_MONTH);
  CAPTURE(c_type);

  // Given a NUMBER column
  conn.execute("CREATE TEMPORARY TABLE t (col NUMBER)");

  // When the year-month interval is bound with SQL_NULL_DATA and inserted
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
