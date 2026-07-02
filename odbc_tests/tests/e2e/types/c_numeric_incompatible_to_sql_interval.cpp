// ODBC E2E: exact-numeric / SQL_C_NUMERIC C types bound via SQLBindParameter to
// the MULTI-FIELD (compound) SQL_INTERVAL_* parameter types must be rejected
// with SQLSTATE 07006 ("Restricted data type attribute violation").
// (SQL_C_BIT is rejected for ALL interval targets; that is a uniform driver
// rejection covered by the Rust unit test interval_tests::bit_to_interval_rejected_07006.)
//
// Per ODBC Appendix D ("C to SQL: Numeric"): "Exact numeric C data types cannot
// be converted to an interval SQL type whose interval precision is not a single
// field." The single-field interval targets ARE permitted and are covered by
// c_integer_conversion_to_sql_interval / c_numeric_conversion_to_sql_interval.
//
// The driver enforces this in interval.rs (render_signed / render_numeric reject
// compound subtypes), so the rejection surfaces from the driver at SQLExecute
// (or SQLBindParameter) before reaching the server.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_SLONG bound to SQL_INTERVAL_YEAR_TO_MONTH",
                 "[c_numeric][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column (sufficient to exercise the bound interval parameter type)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLINTEGER val = 5;
  SQLLEN ind = 0;
  // When a numeric C type is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_SLONG, SQL_INTERVAL_YEAR_TO_MONTH, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_SBIGINT bound to SQL_INTERVAL_DAY_TO_SECOND",
                 "[c_numeric][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLBIGINT val = 9;
  SQLLEN ind = 0;
  // When an exact numeric C type is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_SBIGINT, SQL_INTERVAL_DAY_TO_SECOND, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_SSHORT bound to SQL_INTERVAL_DAY_TO_MINUTE",
                 "[c_numeric][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLSMALLINT val = 4;
  SQLLEN ind = 0;
  // When an exact numeric C type is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_SSHORT, SQL_INTERVAL_DAY_TO_MINUTE, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_UTINYINT bound to SQL_INTERVAL_DAY_TO_HOUR",
                 "[c_numeric][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLCHAR val = 3;
  SQLLEN ind = 0;
  // When an exact numeric C type is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_UTINYINT, SQL_INTERVAL_DAY_TO_HOUR, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_ULONG bound to SQL_INTERVAL_HOUR_TO_SECOND",
                 "[c_numeric][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLUINTEGER val = 6;
  SQLLEN ind = 0;
  // When an exact numeric C type is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_ULONG, SQL_INTERVAL_HOUR_TO_SECOND, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_NUMERIC bound to SQL_INTERVAL_HOUR_TO_MINUTE",
                 "[c_numeric][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 2;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 8);
  SQLLEN ind = sizeof(ns);
  // When a SQL_C_NUMERIC value is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_NUMERIC, SQL_INTERVAL_HOUR_TO_MINUTE, &ns, sizeof(ns), &ind);
}
