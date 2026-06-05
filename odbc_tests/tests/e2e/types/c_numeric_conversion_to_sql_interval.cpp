// ODBC E2E: SQL_C_NUMERIC bound via SQLBindParameter to the single-field
// SQL_INTERVAL_* parameter types.
//
// Per ODBC Appendix D ("C to SQL: Numeric"), SQL_C_NUMERIC (an exact numeric C
// type) may be converted to the single-field interval SQL types (YEAR, MONTH,
// DAY, HOUR, MINUTE, SECOND); the mantissa is interpreted as the count of the
// single leading field. Multi-field/compound interval targets are NOT permitted
// and are covered by c_numeric_incompatible_to_sql_interval.
//
// The conversion under test is keyed by the bound SQL_INTERVAL_* parameter
// type, not the column type, so the parameter is bound as SQL_INTERVAL_* and
// inserted into a VARCHAR column. (Snowflake does have native INTERVAL columns
// as of 2026, but a VARCHAR target is sufficient to exercise the C->SQL
// parameter conversion.) Because the column is VARCHAR, the driver's rendered
// interval literal is stored verbatim, so these tests assert the exact
// round-tripped text; the driver-side formatting itself is additionally pinned
// by the Rust unit tests in interval.rs.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstdint>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

namespace {

SQL_NUMERIC_STRUCT make_numeric(SQLCHAR precision, SQLSCHAR scale, SQLCHAR sign, uint64_t magnitude) {
  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = precision;
  ns.scale = scale;
  ns.sign = sign;  // 1 = positive, 0 = negative
  set_numeric_magnitude(ns, magnitude);
  return ns;
}

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_NUMERIC to SQL_INTERVAL_YEAR",
                 "[c_numeric][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column (sufficient to exercise the bound interval parameter type)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a NUMERIC year count is bound as SQL_INTERVAL_YEAR and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_NUMERIC_STRUCT ns = make_numeric(2, 0, 1, 5);
  SQLLEN ind = sizeof(ns);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTERVAL_YEAR, 0, 0, &ns, sizeof(ns),
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the year count is stored as the interval literal "5"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "5");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_NUMERIC to SQL_INTERVAL_MONTH",
                 "[c_numeric][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a NUMERIC month count is bound as SQL_INTERVAL_MONTH and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_NUMERIC_STRUCT ns = make_numeric(2, 0, 1, 11);
  SQLLEN ind = sizeof(ns);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTERVAL_MONTH, 0, 0, &ns, sizeof(ns),
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the month count is stored as the interval literal "11"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "11");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_NUMERIC to SQL_INTERVAL_SECOND",
                 "[c_numeric][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a NUMERIC second count is bound as SQL_INTERVAL_SECOND and inserted
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_NUMERIC_STRUCT ns = make_numeric(2, 0, 1, 45);
  SQLLEN ind = sizeof(ns);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTERVAL_SECOND, 0, 0, &ns,
                         sizeof(ns), &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the second count is stored as the canonical interval literal "45.000000"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "45.000000");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind fractional SQL_C_NUMERIC (scale>0) to SQL_INTERVAL_YEAR",
                 "[c_numeric][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a NUMERIC(5,2) value of 3.45 (mantissa 345, scale 2) is bound as a
  // SQL_INTERVAL_YEAR count and inserted. A single-field non-SECOND target
  // carries no fractional component, so the driver truncates toward zero.
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_NUMERIC_STRUCT ns = make_numeric(5, 2, 1, 345);
  SQLLEN ind = sizeof(ns);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTERVAL_YEAR, 0, 0, &ns, sizeof(ns),
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then the fractional part is dropped and the year count is stored as "3"
  REQUIRE_ODBC(ret, stmt);
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK(get_data<SQL_C_CHAR>(fetch_stmt, 1) == "3");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should bind SQL_C_NUMERIC with NULL indicator to SQL_INTERVAL_YEAR",
                 "[c_numeric][conversion][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");

  // When a NULL parameter is bound as SQL_INTERVAL_YEAR using SQL_NULL_DATA
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQL_NUMERIC_STRUCT ns = make_numeric(2, 0, 1, 0);
  SQLLEN ind = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_NUMERIC, SQL_INTERVAL_YEAR, 0, 0, &ns, sizeof(ns),
                         &ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the stored value should be NULL
  auto fetch_stmt = conn.execute_fetch("SELECT col FROM t");
  CHECK_FALSE(get_data_optional<SQL_C_CHAR>(fetch_stmt, 1).has_value());
}
