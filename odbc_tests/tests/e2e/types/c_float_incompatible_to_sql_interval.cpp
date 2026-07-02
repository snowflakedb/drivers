// ODBC E2E: approximate-numeric C types (SQL_C_FLOAT / SQL_C_DOUBLE) bound via
// SQLBindParameter to ANY SQL_INTERVAL_* parameter type (single-field or
// compound) must be rejected with SQLSTATE 07006 ("Restricted data type
// attribute violation").
//
// Per ODBC Appendix D ("C to SQL: Numeric"), the numeric-to-interval conversions
// "are not supported for the approximate numeric data types (SQL_C_FLOAT or
// SQL_C_DOUBLE)." Only the exact numeric C types may target single-field
// intervals (see c_integer_conversion_to_sql_interval).
//
// The driver enforces this in interval.rs: SQL_C_FLOAT / SQL_C_DOUBLE have no
// arm in the interval converters and fall through to the 07006 path.

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

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_FLOAT bound to SQL_INTERVAL_YEAR",
                 "[c_float][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER("BD#72",
                  "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind "
                  "semantics");

  // Given a VARCHAR column (sufficient to exercise the bound interval parameter type)
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLREAL val = 5.0f;
  SQLLEN ind = 0;
  // When an approximate numeric C type is bound to a single-field interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_FLOAT, SQL_INTERVAL_YEAR, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_FLOAT bound to SQL_INTERVAL_HOUR",
                 "[c_float][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLREAL val = 12.0f;
  SQLLEN ind = 0;
  // When an approximate numeric C type is bound to a single-field interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_FLOAT, SQL_INTERVAL_HOUR, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_FLOAT bound to SQL_INTERVAL_DAY_TO_SECOND",
                 "[c_float][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLREAL val = 9.0f;
  SQLLEN ind = 0;
  // When an approximate numeric C type is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_FLOAT, SQL_INTERVAL_DAY_TO_SECOND, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_DOUBLE bound to SQL_INTERVAL_MONTH",
                 "[c_float][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLDOUBLE val = 11.0;
  SQLLEN ind = 0;
  // When an approximate numeric C type is bound to a single-field interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_DOUBLE, SQL_INTERVAL_MONTH, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_DOUBLE bound to SQL_INTERVAL_SECOND",
                 "[c_float][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLDOUBLE val = 45.0;
  SQLLEN ind = 0;
  // When an approximate numeric C type is bound to a single-field interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_DOUBLE, SQL_INTERVAL_SECOND, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_DOUBLE bound to SQL_INTERVAL_YEAR_TO_MONTH",
                 "[c_float][incompatible][sql_interval]") {
  SKIP_OLD_DRIVER(
      "BD#72",
      "Reference driver does not implement ODBC Appendix D numeric C -> single-field SQL_INTERVAL bind semantics");
  conn.execute("CREATE TEMPORARY TABLE t (col VARCHAR(200))");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQLDOUBLE val = 5.0;
  SQLLEN ind = 0;
  // When an approximate numeric C type is bound to a compound interval target
  // Then the driver rejects the conversion with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_DOUBLE, SQL_INTERVAL_YEAR_TO_MONTH, &val, 0, &ind);
}
