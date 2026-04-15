// Tests that binding numeric and bit C types to SQL_TYPE_TIMESTAMP returns an error,
// as these conversions are not listed in the ODBC spec conversion table
// (Appendix D, "C to SQL: Timestamp"). Only SQL_C_CHAR, SQL_C_WCHAR,
// SQL_C_TYPE_DATE, and SQL_C_TYPE_TIMESTAMP may be bound to SQL_TYPE_TIMESTAMP.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should reject SQL_C_SLONG bound to SQL_TYPE_TIMESTAMP", "[c_numeric][incompatible][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP)");

  SQLINTEGER val = 20250115;
  SQLLEN ind = 0;

  // When SQL_C_SLONG is bound to SQL_TYPE_TIMESTAMP and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_SLONG, SQL_TYPE_TIMESTAMP, &val, 0, &ind);
}

TEST_CASE("should reject SQL_C_DOUBLE bound to SQL_TYPE_TIMESTAMP", "[c_numeric][incompatible][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP)");

  SQLDOUBLE val = 20250115.0;
  SQLLEN ind = 0;

  // When SQL_C_DOUBLE is bound to SQL_TYPE_TIMESTAMP and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_DOUBLE, SQL_TYPE_TIMESTAMP, &val, 0, &ind);
}

TEST_CASE("should reject SQL_C_FLOAT bound to SQL_TYPE_TIMESTAMP", "[c_numeric][incompatible][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP)");

  SQLREAL val = 20250115.0f;
  SQLLEN ind = 0;

  // When SQL_C_FLOAT is bound to SQL_TYPE_TIMESTAMP and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_FLOAT, SQL_TYPE_TIMESTAMP, &val, 0, &ind);
}

TEST_CASE("should reject SQL_C_BIT bound to SQL_TYPE_TIMESTAMP", "[c_numeric][incompatible][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP)");

  SQLCHAR val = 1;
  SQLLEN ind = 0;

  // When SQL_C_BIT is bound to SQL_TYPE_TIMESTAMP and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_BIT, SQL_TYPE_TIMESTAMP, &val, 0, &ind);
}

TEST_CASE("should reject SQL_C_NUMERIC bound to SQL_TYPE_TIMESTAMP", "[c_numeric][incompatible][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP)");

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 8;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 20250115);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC is bound to SQL_TYPE_TIMESTAMP and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_NUMERIC, SQL_TYPE_TIMESTAMP, &ns, sizeof(ns), &ind);
}

TEST_CASE("should reject SQL_C_SBIGINT bound to SQL_TYPE_TIMESTAMP", "[c_numeric][incompatible][sql_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col TIMESTAMP)");

  SQLBIGINT val = 20250115;
  SQLLEN ind = 0;

  // When SQL_C_SBIGINT is bound to SQL_TYPE_TIMESTAMP and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_SBIGINT, SQL_TYPE_TIMESTAMP, &val, 0, &ind);
}
