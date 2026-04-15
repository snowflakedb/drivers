// Tests that binding temporal and GUID C types to fixed-point SQL types
// (SQL_INTEGER, SQL_BIGINT) returns an error, as these conversions are
// not listed in the ODBC spec conversion table (Appendix D, "C to SQL: Numeric").

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("should reject SQL_C_TYPE_DATE bound to SQL_INTEGER", "[c_temporal][incompatible][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  SQL_DATE_STRUCT ds = {2025, 1, 15};
  SQLLEN ind = sizeof(ds);

  // When SQL_C_TYPE_DATE is bound to SQL_INTEGER and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_TYPE_DATE, SQL_INTEGER, &ds, sizeof(ds), &ind);
}

TEST_CASE("should reject SQL_C_TYPE_TIME bound to SQL_INTEGER", "[c_temporal][incompatible][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  SQL_TIME_STRUCT ts = {12, 30, 45};
  SQLLEN ind = sizeof(ts);

  // When SQL_C_TYPE_TIME is bound to SQL_INTEGER and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_TYPE_TIME, SQL_INTEGER, &ts, sizeof(ts), &ind);
}

TEST_CASE("should reject SQL_C_TYPE_TIMESTAMP bound to SQL_INTEGER", "[c_temporal][incompatible][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  SQL_TIMESTAMP_STRUCT tss = {2025, 1, 15, 12, 30, 45, 0};
  SQLLEN ind = sizeof(tss);

  // When SQL_C_TYPE_TIMESTAMP is bound to SQL_INTEGER and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_TYPE_TIMESTAMP, SQL_INTEGER, &tss, sizeof(tss), &ind);
}

TEST_CASE("should reject SQL_C_GUID bound to SQL_INTEGER", "[c_temporal][incompatible][sql_fixed]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);
  conn.execute("CREATE TABLE t (col NUMBER)");

  SQLGUID guid = {0x12345678, 0x1234, 0x5678, {0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78}};
  SQLLEN ind = sizeof(guid);

  // When SQL_C_GUID is bound to SQL_INTEGER and executed
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO t VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the driver rejects the incompatible conversion with an error
  check_incompatible_bindparam(stmt, SQL_C_GUID, SQL_INTEGER, &guid, sizeof(guid), &ind);
}
