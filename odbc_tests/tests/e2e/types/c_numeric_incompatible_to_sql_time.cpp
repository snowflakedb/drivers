// Tests that binding numeric and bit C types to SQL_TYPE_TIME returns an error,
// as these conversions are not listed in the ODBC spec conversion table
// (Appendix D, "C to SQL: Time"). Only SQL_C_CHAR, SQL_C_WCHAR,
// SQL_C_TYPE_TIME, and SQL_C_TYPE_TIMESTAMP may be bound to SQL_TYPE_TIME.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_SLONG bound to SQL_TYPE_TIME",
                 "[c_numeric][incompatible][sql_time]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  SQLINTEGER val = 123045;
  SQLLEN ind = 0;

  // When SQL_C_SLONG is bound to SQL_TYPE_TIME and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_SLONG, SQL_TYPE_TIME, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_DOUBLE bound to SQL_TYPE_TIME",
                 "[c_numeric][incompatible][sql_time]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  SQLDOUBLE val = 123045.0;
  SQLLEN ind = 0;

  // When SQL_C_DOUBLE is bound to SQL_TYPE_TIME and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_DOUBLE, SQL_TYPE_TIME, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_FLOAT bound to SQL_TYPE_TIME",
                 "[c_numeric][incompatible][sql_time]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  SQLREAL val = 123045.0f;
  SQLLEN ind = 0;

  // When SQL_C_FLOAT is bound to SQL_TYPE_TIME and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_FLOAT, SQL_TYPE_TIME, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BIT bound to SQL_TYPE_TIME",
                 "[c_numeric][incompatible][sql_time]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT is bound to SQL_TYPE_TIME and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_BIT, SQL_TYPE_TIME, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_NUMERIC bound to SQL_TYPE_TIME",
                 "[c_numeric][incompatible][sql_time]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 6;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 123045);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC is bound to SQL_TYPE_TIME and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_NUMERIC, SQL_TYPE_TIME, &ns, sizeof(ns), &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_SBIGINT bound to SQL_TYPE_TIME",
                 "[c_numeric][incompatible][sql_time]") {
  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col TIME)");

  SQLBIGINT val = 123045;
  SQLLEN ind = 0;

  // When SQL_C_SBIGINT is bound to SQL_TYPE_TIME and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_SBIGINT, SQL_TYPE_TIME, &val, 0, &ind);
}
