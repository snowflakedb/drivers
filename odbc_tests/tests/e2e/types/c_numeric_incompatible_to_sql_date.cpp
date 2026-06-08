// Tests that binding numeric and bit C types to a DATE target returns an
// error, as these conversions are not listed in the ODBC spec conversion
// table (Appendix D, "C to SQL: Date"). Only SQL_C_CHAR, SQL_C_WCHAR,
// SQL_C_TYPE_DATE, and SQL_C_TYPE_TIMESTAMP may be bound to a DATE target.
//
// Per ODBC Appendix G ("Driver Guidelines for Backward Compatibility"),
// the ODBC 3.x code SQL_TYPE_DATE (91) and its ODBC 2.x predecessor
// SQL_DATE (9) must be accepted as identical at the SQLBindParameter
// boundary, including for the negative path tested here — both spellings
// must reject the same set of incompatible C source types in the same
// way. Each TEST_CASE below is parametrized over both spellings using
// Catch2 GENERATE.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_SLONG bound to DATE target",
                 "[c_numeric][incompatible][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  SQLINTEGER val = 20250115;
  SQLLEN ind = 0;

  // When SQL_C_SLONG is bound to the DATE target and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_SLONG, sql_type, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_DOUBLE bound to DATE target",
                 "[c_numeric][incompatible][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  SQLDOUBLE val = 20250115.0;
  SQLLEN ind = 0;

  // When SQL_C_DOUBLE is bound to the DATE target and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_DOUBLE, sql_type, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_FLOAT bound to DATE target",
                 "[c_numeric][incompatible][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  SQLREAL val = 20250115.0f;
  SQLLEN ind = 0;

  // When SQL_C_FLOAT is bound to the DATE target and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_FLOAT, sql_type, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_BIT bound to DATE target",
                 "[c_numeric][incompatible][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT is bound to the DATE target and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_BIT, sql_type, &val, 0, &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_NUMERIC bound to DATE target",
                 "[c_numeric][incompatible][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 8;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 20250115);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC is bound to the DATE target and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_NUMERIC, sql_type, &ns, sizeof(ns), &ind);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject SQL_C_SBIGINT bound to DATE target",
                 "[c_numeric][incompatible][sql_date]") {
  const SQLSMALLINT sql_type = GENERATE(SQL_DATE, SQL_TYPE_DATE);
  CAPTURE(sql_type);

  // Given Snowflake client is logged in
  conn.execute("CREATE TEMPORARY TABLE t (col DATE)");

  SQLBIGINT val = 20250115;
  SQLLEN ind = 0;

  // When SQL_C_SBIGINT is bound to the DATE target and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_SBIGINT, sql_type, &val, 0, &ind);
}
