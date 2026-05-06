// Negative bindparam coverage for SQL_BINARY / SQL_VARBINARY /
// SQL_LONGVARBINARY targets. Per ODBC Appendix D ("Converting Data
// from C to SQL Data Types", section "Binary"), the only legal C
// source types are SQL_C_BINARY, SQL_C_CHAR, SQL_C_WCHAR, and
// SQL_C_DEFAULT. Every other C type must surface as SQLSTATE 07006
// ("Restricted data type attribute violation") — either at
// SQLBindParameter time (when the Driver Manager filters via
// SQL_CONVERT_<source>) or at SQLExecute time (when the driver's
// SnowflakeBinary::read_odbc rejects the C type).
//
// This file is the bind-direction mirror of
// `binary_incompatible_conversions.cpp` (which exercises the fetch
// direction Snowflake BINARY → C type).

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "SchemaFixtures.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// Helper: prepare an INSERT into a 1-column BINARY temp table
// ============================================================================

namespace {
HandleWrapper prepare_binary_insert(Connection& conn) {
  conn.execute("CREATE OR REPLACE TEMPORARY TABLE cm_binary (val BINARY)");
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("INSERT INTO cm_binary VALUES (?)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  return stmt;
}
}  // namespace

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Integer C types -> SQL_BINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding integer C types to SQL_BINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  // Given Snowflake client is logged in
  // And a temporary BINARY column exists
  auto stmt = prepare_binary_insert(conn);

  // Then SQL_C_BIT is rejected with SQLSTATE 07006
  {
    SQLCHAR v = 1;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_BIT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_TINYINT
  {
    SQLSCHAR v = 1;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_TINYINT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_SHORT
  {
    SQLSMALLINT v = 1;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_SHORT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_LONG
  {
    SQLINTEGER v = 12345;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_LONG, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_SBIGINT
  {
    SQLBIGINT v = 12345;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_SBIGINT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_UBIGINT
  {
    SQLUBIGINT v = 12345;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_UBIGINT, SQL_BINARY, &v, sizeof(v), &ind);
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Floating-point C types -> SQL_VARBINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding floating-point C types to SQL_VARBINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  auto stmt = prepare_binary_insert(conn);

  {
    SQLREAL v = 1.5f;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_FLOAT, SQL_VARBINARY, &v, sizeof(v), &ind);
  }
  {
    SQLDOUBLE v = 1.5;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_DOUBLE, SQL_VARBINARY, &v, sizeof(v), &ind);
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - SQL_C_NUMERIC -> SQL_LONGVARBINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding SQL_C_NUMERIC to SQL_LONGVARBINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  auto stmt = prepare_binary_insert(conn);

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 5;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 12345);
  SQLLEN ind = sizeof(ns);
  check_incompatible_bindparam(stmt, SQL_C_NUMERIC, SQL_LONGVARBINARY, &ns, sizeof(ns), &ind);
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Temporal C types -> SQL_BINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding temporal C types to SQL_BINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  auto stmt = prepare_binary_insert(conn);

  {
    SQL_DATE_STRUCT v = {2026, 1, 1};
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_TYPE_DATE, SQL_BINARY, &v, sizeof(v), &ind);
  }
  {
    SQL_TIME_STRUCT v = {12, 30, 0};
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_TYPE_TIME, SQL_BINARY, &v, sizeof(v), &ind);
  }
  {
    SQL_TIMESTAMP_STRUCT v = {2026, 1, 1, 12, 30, 0, 0};
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_TYPE_TIMESTAMP, SQL_BINARY, &v, sizeof(v), &ind);
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Single-component interval C types -> SQL_BINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding single-component interval C types to SQL_BINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  auto stmt = prepare_binary_insert(conn);

  for (SQLSMALLINT c_type : {SQL_C_INTERVAL_YEAR, SQL_C_INTERVAL_MONTH, SQL_C_INTERVAL_DAY, SQL_C_INTERVAL_HOUR,
                             SQL_C_INTERVAL_MINUTE, SQL_C_INTERVAL_SECOND}) {
    SQL_INTERVAL_STRUCT v = {};
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, c_type, SQL_BINARY, &v, sizeof(v), &ind);
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Compound interval C types -> SQL_VARBINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding compound interval C types to SQL_VARBINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  auto stmt = prepare_binary_insert(conn);

  for (SQLSMALLINT c_type : {SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_C_INTERVAL_DAY_TO_HOUR, SQL_C_INTERVAL_DAY_TO_MINUTE,
                             SQL_C_INTERVAL_DAY_TO_SECOND, SQL_C_INTERVAL_HOUR_TO_MINUTE,
                             SQL_C_INTERVAL_HOUR_TO_SECOND, SQL_C_INTERVAL_MINUTE_TO_SECOND}) {
    SQL_INTERVAL_STRUCT v = {};
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, c_type, SQL_VARBINARY, &v, sizeof(v), &ind);
  }
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - SQL_C_GUID -> SQL_LONGVARBINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding SQL_C_GUID to SQL_LONGVARBINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  auto stmt = prepare_binary_insert(conn);

  SQLGUID v = {};
  SQLLEN ind = sizeof(v);
  check_incompatible_bindparam(stmt, SQL_C_GUID, SQL_LONGVARBINARY, &v, sizeof(v), &ind);
}

// ============================================================================
// POSITIVE PATH - the four legal source types must continue to work
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should accept SQL_C_BINARY, SQL_C_CHAR, SQL_C_WCHAR, SQL_C_DEFAULT to SQL_BINARY",
                 "[c_to_binary][bindparam][positive]") {
  auto stmt = prepare_binary_insert(conn);

  // SQL_C_BINARY: raw bytes verbatim
  {
    SQLCHAR raw[] = {0xDE, 0xAD, 0xBE, 0xEF};
    SQLLEN ind = sizeof(raw);
    SQLRETURN ret =
        SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, 0, 0, raw, sizeof(raw), &ind);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(SQLExecute(stmt.getHandle()) == SQL_SUCCESS);
  }

  // SQL_C_CHAR: ASCII hex literal "DEADBEEF" must hex-decode to 4 bytes
  {
    SQLCHAR hex_lit[] = "DEADBEEF";
    SQLLEN ind = SQL_NTS;
    SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BINARY, 0, 0, hex_lit, 0,
                                     &ind);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(SQLExecute(stmt.getHandle()) == SQL_SUCCESS);
  }
}
