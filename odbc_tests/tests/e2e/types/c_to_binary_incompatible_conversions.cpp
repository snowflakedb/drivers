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
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  // When integer C types are bound to SQL_BINARY and executed
  // Then SQL_C_BIT bind is rejected with SQLSTATE 07006
  {
    SQLCHAR v = 1;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_BIT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_TINYINT bind is rejected with SQLSTATE 07006
  {
    SQLSCHAR v = 1;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_TINYINT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_SHORT bind is rejected with SQLSTATE 07006
  {
    SQLSMALLINT v = 1;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_SHORT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_LONG bind is rejected with SQLSTATE 07006
  {
    SQLINTEGER v = 12345;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_LONG, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_SBIGINT bind is rejected with SQLSTATE 07006
  {
    SQLBIGINT v = 12345;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_SBIGINT, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_UBIGINT bind is rejected with SQLSTATE 07006
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
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  // When floating-point C types are bound to SQL_VARBINARY and executed
  // Then SQL_C_FLOAT bind is rejected with SQLSTATE 07006
  {
    SQLREAL v = 1.5f;
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_FLOAT, SQL_VARBINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_DOUBLE bind is rejected with SQLSTATE 07006
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
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  SQL_NUMERIC_STRUCT ns = {};
  ns.precision = 5;
  ns.scale = 0;
  ns.sign = 1;
  set_numeric_magnitude(ns, 12345);
  SQLLEN ind = sizeof(ns);

  // When SQL_C_NUMERIC is bound to SQL_LONGVARBINARY and executed
  // Then the bind is rejected with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_NUMERIC, SQL_LONGVARBINARY, &ns, sizeof(ns), &ind);
}

// ============================================================================
// INCOMPATIBLE CONVERSIONS - Temporal C types -> SQL_BINARY
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail binding temporal C types to SQL_BINARY",
                 "[c_to_binary][bindparam][incompatible][negative]") {
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  // When temporal C types are bound to SQL_BINARY and executed
  // Then SQL_C_TYPE_DATE bind is rejected with SQLSTATE 07006
  {
    SQL_DATE_STRUCT v = {2026, 1, 1};
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_TYPE_DATE, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_TYPE_TIME bind is rejected with SQLSTATE 07006
  {
    SQL_TIME_STRUCT v = {12, 30, 0};
    SQLLEN ind = sizeof(v);
    check_incompatible_bindparam(stmt, SQL_C_TYPE_TIME, SQL_BINARY, &v, sizeof(v), &ind);
  }
  // And SQL_C_TYPE_TIMESTAMP bind is rejected with SQLSTATE 07006
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
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  // When each single-component SQL_C_INTERVAL_* type is bound to SQL_BINARY and executed
  // Then every interval bind is rejected with SQLSTATE 07006
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
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  // When each compound SQL_C_INTERVAL_* type is bound to SQL_VARBINARY and executed
  // Then every interval bind is rejected with SQLSTATE 07006
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
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  SQLGUID v = {};
  SQLLEN ind = sizeof(v);

  // When SQL_C_GUID is bound to SQL_LONGVARBINARY and executed
  // Then the bind is rejected with SQLSTATE 07006
  check_incompatible_bindparam(stmt, SQL_C_GUID, SQL_LONGVARBINARY, &v, sizeof(v), &ind);
}

// ============================================================================
// POSITIVE PATH - the four legal source types must continue to work
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should accept SQL_C_BINARY, SQL_C_CHAR, SQL_C_WCHAR, SQL_C_DEFAULT to SQL_BINARY",
                 "[c_to_binary][bindparam][positive]") {
  // Given a temporary BINARY column exists and an INSERT statement is prepared
  auto stmt = prepare_binary_insert(conn);

  // When SQL_C_BINARY is bound to SQL_BINARY with raw bytes and executed
  // Then the bind succeeds and the bytes land in Snowflake verbatim
  {
    SQLCHAR raw[] = {0xDE, 0xAD, 0xBE, 0xEF};
    SQLLEN ind = sizeof(raw);
    SQLRETURN ret =
        SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_BINARY, SQL_BINARY, 0, 0, raw, sizeof(raw), &ind);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(SQLExecute(stmt.getHandle()) == SQL_SUCCESS);
  }

  // And when SQL_C_CHAR is bound to SQL_BINARY with the ASCII hex literal "DEADBEEF"
  // Then the bind succeeds and the driver hex-decodes the literal into 4 bytes server-side
  {
    SQLCHAR hex_lit[] = "DEADBEEF";
    SQLLEN ind = SQL_NTS;
    SQLRETURN ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_BINARY, 0, 0, hex_lit, 0,
                                     &ind);
    REQUIRE_ODBC(ret, stmt);
    REQUIRE(SQLExecute(stmt.getHandle()) == SQL_SUCCESS);
  }
}
